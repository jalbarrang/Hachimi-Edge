//! Persisted overlay presentation prefs (currently just the content zoom).
//!
//! The zoom is a uniform multiplier applied to the overlay's font + spacing so
//! the whole panel scales. It is an explicit user setting (slider) rather than
//! being derived from the window size, which would feed back into the panel's
//! auto-sizing and grow unbounded.

use std::sync::atomic::{AtomicU32, Ordering};

pub(crate) const MIN_ZOOM: f32 = 0.7;
pub(crate) const MAX_ZOOM: f32 = 2.5;
const DEFAULT_ZOOM: f32 = 1.0;

static ZOOM: AtomicU32 = AtomicU32::new(DEFAULT_ZOOM.to_bits());

/// Current overlay content zoom (clamped to `[MIN_ZOOM, MAX_ZOOM]`).
pub(crate) fn zoom() -> f32 {
    f32::from_bits(ZOOM.load(Ordering::Relaxed))
}

/// Set the overlay content zoom (callers persist afterwards).
pub(crate) fn set_zoom(value: f32) {
    ZOOM.store(value.clamp(MIN_ZOOM, MAX_ZOOM).to_bits(), Ordering::Relaxed);
}

/// Default zoom for fresh configs.
pub(crate) fn default_zoom() -> f32 {
    DEFAULT_ZOOM
}
