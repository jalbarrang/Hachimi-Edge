//! In-panel tab selection (in-memory only; resets on reload).

use std::sync::atomic::Ordering;

/// In-panel tabs for the overlay.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(super) enum Tab {
    Training = 0,
    Skills = 1,
    Shop = 3,
    Scenario = 4,
}

static SELECTED_TAB: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

pub(super) fn selected_tab() -> Tab {
    match SELECTED_TAB.load(Ordering::Relaxed) {
        1 => Tab::Skills,
        3 => Tab::Shop,
        4 => Tab::Scenario,
        _ => Tab::Training,
    }
}

pub(super) fn set_selected_tab(tab: Tab) {
    SELECTED_TAB.store(tab as u8, Ordering::Relaxed);
}
