//! Shared plugin state: decoded race + names + watch filter + live snapshot.

use std::sync::{Mutex, OnceLock};

use crate::sim::{DecodedRace, RaceSummary};

static STATE: OnceLock<Mutex<State>> = OnceLock::new();

/// One runner row in the live feed (sorted by distance, leader first).
#[derive(Clone, Debug)]
pub struct RunnerRow {
    pub rank: u8,
    pub post: u8,
    #[allow(dead_code)]
    pub name: String,
    pub distance: f32,
    pub speed: u16,
    pub hp: u16,
    #[allow(dead_code)]
    pub temptation: i8,
}

/// Latest sampled race state for the overlay.
#[derive(Clone, Debug)]
pub struct LiveSnapshot {
    pub elapsed: f32,
    pub frame_index: usize,
    pub frame_count: usize,
    pub rows: Vec<RunnerRow>,
}

/// One entry in the watch-selection list (post #, name, watched flag).
#[derive(Clone, Debug)]
pub struct WatchEntry {
    pub post: u8,
    #[allow(dead_code)]
    pub name: String,
    pub watched: bool,
}

/// Read-only view assembled for the overlay.
#[derive(Clone, Debug, Default)]
pub struct UiState {
    pub captured: bool,
    pub capture_count: u64,
    pub summary: Option<RaceSummary>,
    pub watch: Vec<WatchEntry>,
    pub live: Option<LiveSnapshot>,
}

#[derive(Debug, Default)]
struct State {
    /// `(race_info_addr, simdata_len)` of the last capture (dedupe).
    signature: Option<(usize, i32)>,
    capture_count: u64,
    summary: Option<RaceSummary>,
    frames: Vec<crate::sim::FrameData>,
    /// Character names by horse index (may be empty if unavailable).
    names: Vec<String>,
    /// Watch flag by horse index (true = show in the live grid).
    watch: Vec<bool>,
    live: Option<LiveSnapshot>,
}

fn cell() -> &'static Mutex<State> {
    STATE.get_or_init(|| Mutex::new(State::default()))
}

pub fn init() {
    let _ = cell();
}

/// Whether `(addr, len)` differs from the last capture (cheap pre-decode check).
#[must_use]
pub fn is_new_signature(race_info_addr: usize, simdata_len: i32) -> bool {
    cell()
        .lock()
        .expect("race-hud state lock poisoned")
        .signature
        .is_none_or(|s| s != (race_info_addr, simdata_len))
}

/// Store a freshly decoded race (frames + per-runner names).
pub fn set_decoded(race_info_addr: usize, simdata_len: i32, decoded: Option<DecodedRace>, names: Vec<String>) {
    let mut state = cell().lock().expect("race-hud state lock poisoned");
    state.signature = Some((race_info_addr, simdata_len));
    state.capture_count += 1;
    state.live = None;
    state.names = names;
    match decoded {
        Some(d) => {
            let count = d.summary.horse_num.max(0) as usize;
            state.watch = vec![true; count];
            state.summary = Some(d.summary);
            state.frames = d.frames;
        }
        None => {
            state.summary = None;
            state.frames.clear();
            state.watch.clear();
        }
    }
}

/// Sample the decoded frames at race time `elapsed`, refreshing the live snapshot.
pub fn sample_live(elapsed: f32) {
    let mut state = cell().lock().expect("race-hud state lock poisoned");
    if state.frames.is_empty() {
        return;
    }

    let idx = state.frames.partition_point(|f| f.time <= elapsed).saturating_sub(1);

    let mut rows: Vec<RunnerRow> = state.frames[idx]
        .runners
        .iter()
        .enumerate()
        .map(|(i, r)| RunnerRow {
            rank: 0,
            post: (i + 1) as u8,
            name: state.names.get(i).cloned().unwrap_or_default(),
            distance: r.distance,
            speed: r.speed,
            hp: r.hp,
            temptation: r.temptation,
        })
        .collect();

    rows.sort_by(|a, b| b.distance.total_cmp(&a.distance));
    for (i, row) in rows.iter_mut().enumerate() {
        row.rank = (i + 1) as u8;
    }

    let frame_count = state.frames.len();
    state.live = Some(LiveSnapshot {
        elapsed,
        frame_index: idx,
        frame_count,
        rows,
    });
}

/// Toggle the watch flag for a 1-based post number.
pub fn toggle_watch(post: u8) {
    let mut state = cell().lock().expect("race-hud state lock poisoned");
    if let Some(flag) = state.watch.get_mut((post as usize).wrapping_sub(1)) {
        *flag = !*flag;
    }
}

/// Set every runner's watch flag at once (Show all / Hide all helpers).
pub fn set_all_watched(watched: bool) {
    let mut state = cell().lock().expect("race-hud state lock poisoned");
    for flag in &mut state.watch {
        *flag = watched;
    }
}

#[must_use]
pub fn ui_state() -> UiState {
    let state = cell().lock().expect("race-hud state lock poisoned");
    let watch = state
        .watch
        .iter()
        .enumerate()
        .map(|(i, &watched)| WatchEntry {
            post: (i + 1) as u8,
            name: state.names.get(i).cloned().unwrap_or_default(),
            watched,
        })
        .collect();

    // Filter the live rows to watched runners (if every runner is hidden, show all
    // so the grid is never mysteriously empty).
    let any_watched = state.watch.iter().any(|&w| w);
    let live = state.live.as_ref().map(|snap| {
        let rows = snap
            .rows
            .iter()
            .filter(|r| !any_watched || *state.watch.get((r.post as usize).wrapping_sub(1)).unwrap_or(&true))
            .cloned()
            .collect();
        LiveSnapshot {
            elapsed: snap.elapsed,
            frame_index: snap.frame_index,
            frame_count: snap.frame_count,
            rows,
        }
    });

    UiState {
        captured: state.signature.is_some(),
        capture_count: state.capture_count,
        summary: state.summary,
        watch,
        live,
    }
}

/// Reset everything (manual Reset button or shutdown).
pub fn clear_all() {
    let mut state = cell().lock().expect("race-hud state lock poisoned");
    *state = State::default();
}
