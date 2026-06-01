//! Host→plugin event dispatch.
//!
//! Plugins subscribe via the `host_subscribe` vtable slot; the host fires events
//! from its own lifecycle (per-frame, config reload, shutdown). Callbacks are
//! snapshotted under a short lock and invoked with the lock released, each wrapped
//! in `catch_unwind` so a misbehaving plugin can't take down the host thread.

use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Mutex;

use hachimi_plugin_abi::event;
use hachimi_plugin_abi::{PluginEventFn, TrainingCommandEvent, ViewChangeEvent};
use once_cell::sync::Lazy;

use super::{current_owner, next_handle, OwnerScope};

struct Subscription {
    handle: u64,
    owner: u32,
    event_id: u32,
    callback: PluginEventFn,
    userdata: usize,
}

static SUBSCRIPTIONS: Lazy<Mutex<Vec<Subscription>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// Register an event callback. Returns a non-zero handle.
pub fn subscribe(event_id: u32, callback: PluginEventFn, userdata: *mut c_void) -> u64 {
    let handle = next_handle();
    SUBSCRIPTIONS.lock().expect("lock poisoned").push(Subscription {
        handle,
        owner: current_owner(),
        event_id,
        callback,
        userdata: userdata as usize,
    });
    handle
}

/// Remove a subscription by handle.
pub fn unsubscribe(handle: u64) {
    SUBSCRIPTIONS
        .lock()
        .expect("lock poisoned")
        .retain(|s| s.handle != handle);
}

/// Invoke every callback registered for `event_id`. `data` is event-specific.
pub fn dispatch(event_id: u32, data: *const c_void) {
    // Snapshot matching callbacks, then release the lock before invoking so a
    // callback may safely (un)subscribe or call back into the host.
    let targets: Vec<(u32, PluginEventFn, usize)> = {
        let subs = SUBSCRIPTIONS.lock().expect("lock poisoned");
        if subs.is_empty() {
            return;
        }
        subs.iter()
            .filter(|s| s.event_id == event_id)
            .map(|s| (s.owner, s.callback, s.userdata))
            .collect()
    };

    for (owner, callback, userdata) in targets {
        // Attribute any registrations made from inside the callback to its plugin.
        let _scope = OwnerScope::enter(owner);
        let _ = catch_unwind(AssertUnwindSafe(|| callback(event_id, data, userdata as *mut c_void)))
            .inspect_err(|_| error!("plugin event callback panicked (event {})", event_id));
    }
}

/// Dispatch `SHUTDOWN` to a single plugin's subscriptions, then drop all of that
/// plugin's subscriptions. Used when unloading one plugin.
pub(crate) fn shutdown_and_remove_owner(owner: u32) {
    let targets: Vec<(PluginEventFn, usize)> = {
        let subs = SUBSCRIPTIONS.lock().expect("lock poisoned");
        subs.iter()
            .filter(|s| s.owner == owner)
            .map(|s| (s.callback, s.userdata))
            .collect()
    };

    for (callback, userdata) in targets {
        let _ = catch_unwind(AssertUnwindSafe(|| {
            callback(event::SHUTDOWN, std::ptr::null(), userdata as *mut c_void)
        }))
        .inspect_err(|_| error!("plugin SHUTDOWN callback panicked (owner {})", owner));
    }

    SUBSCRIPTIONS
        .lock()
        .expect("lock poisoned")
        .retain(|s| s.owner != owner);
}

/// Fired once per rendered frame.
pub fn dispatch_frame() {
    dispatch(event::FRAME, std::ptr::null());
}

/// Fired after the host reloads its config.
pub fn dispatch_config_reload() {
    dispatch(event::CONFIG_RELOAD, std::ptr::null());
}

/// Fired before the host unloads.
pub fn dispatch_shutdown() {
    dispatch(event::SHUTDOWN, std::ptr::null());
}

/// Fired when the game changes view/scene.
pub fn dispatch_view_change(view_id: i32) {
    let payload = ViewChangeEvent { view_id };
    dispatch(event::VIEW_CHANGE, std::ptr::from_ref(&payload).cast());
}

/// Fired once when the splash screen is first shown (game ready).
pub fn dispatch_splash_shown() {
    dispatch(event::SPLASH_SHOWN, std::ptr::null());
}

/// Fired when a career run becomes active.
pub fn dispatch_career_start() {
    dispatch(event::CAREER_START, std::ptr::null());
}

/// Fired when a career run ends.
pub fn dispatch_career_end() {
    dispatch(event::CAREER_END, std::ptr::null());
}

/// Fired when the player submits a training command.
pub fn dispatch_training_command(command_id: i32) {
    let payload = TrainingCommandEvent { command_id };
    dispatch(event::TRAINING_COMMAND, std::ptr::from_ref(&payload).cast());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::super::TEST_LOCK;
    static SHUTDOWN_HITS: AtomicU32 = AtomicU32::new(0);

    extern "C" fn noop(_e: u32, _d: *const c_void, _u: *mut c_void) {}
    extern "C" fn count_shutdown(event_id: u32, _d: *const c_void, _u: *mut c_void) {
        if event_id == event::SHUTDOWN {
            SHUTDOWN_HITS.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn clear() {
        SUBSCRIPTIONS.lock().expect("lock poisoned").clear();
    }

    #[test]
    fn shutdown_and_remove_owner_is_scoped() {
        let _guard = TEST_LOCK.lock().expect("lock poisoned");
        clear();
        SHUTDOWN_HITS.store(0, Ordering::Relaxed);

        // Two subscriptions for owner 5, one for owner 6.
        {
            let _s = OwnerScope::enter(5);
            subscribe(event::FRAME, count_shutdown, std::ptr::null_mut());
            subscribe(event::CAREER_START, count_shutdown, std::ptr::null_mut());
        }
        {
            let _s = OwnerScope::enter(6);
            subscribe(event::FRAME, noop, std::ptr::null_mut());
        }
        assert_eq!(SUBSCRIPTIONS.lock().expect("lock poisoned").len(), 3);

        shutdown_and_remove_owner(5);

        // Both owner-5 callbacks received SHUTDOWN, and only owner 6 remains.
        assert_eq!(SHUTDOWN_HITS.load(Ordering::Relaxed), 2);
        let subs = SUBSCRIPTIONS.lock().expect("lock poisoned");
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].owner, 6);
        drop(subs);
        clear();
    }

    #[test]
    fn subscribe_tags_current_owner() {
        let _guard = TEST_LOCK.lock().expect("lock poisoned");
        clear();
        let _s = OwnerScope::enter(42);
        subscribe(event::FRAME, noop, std::ptr::null_mut());
        assert_eq!(SUBSCRIPTIONS.lock().expect("lock poisoned")[0].owner, 42);
        drop(_s);
        clear();
    }
}
