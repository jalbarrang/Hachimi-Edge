//! Hooks the career-mode command submission so the host can emit
//! [`event::TRAINING_COMMAND`] to subscribed plugins.
//!
//! `SingleModeMainViewController.SendCommandAsync(6)` carries the chosen
//! `command_id` as its first argument (runtime-verified: `command_id=101` for
//! Speed). Per the IL2CPP dump its real signature is:
//!
//! ```text
//! System.Collections.IEnumerator SendCommandAsync(6 args)
//! ```
//!
//! It is a **coroutine kickoff**: it returns an `IEnumerator` that the caller
//! feeds to `StartCoroutine`. The hook MUST forward that return value — declaring
//! a `void` hook leaves garbage in the return register and the game crashes when it
//! starts the coroutine. We only read `command_id` and otherwise pass everything
//! through unchanged.

use crate::il2cpp::{symbols::get_method_addr, types::*};

type SendCommandAsyncFn = extern "C" fn(
    this: *mut Il2CppObject,
    command_id: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
) -> *mut Il2CppObject;

extern "C" fn SendCommandAsync(
    this: *mut Il2CppObject,
    command_id: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
) -> *mut Il2CppObject {
    crate::core::plugin::events::dispatch_training_command(command_id as i32);
    get_orig_fn!(SendCommandAsync, SendCommandAsyncFn)(this, command_id, a2, a3, a4, a5, a6)
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, SingleModeMainViewController);

    let SendCommandAsync_addr = get_method_addr(SingleModeMainViewController, c"SendCommandAsync", 6);
    new_hook!(SendCommandAsync_addr, SendCommandAsync);
}
