//! Shared UI sizing and overlay identifiers.

/// Default overlay font size.
pub(super) const OVERLAY_FONT_SIZE: f32 = 12.0;
pub(super) const OVERLAY_MIN_WIDTH: f32 = 340.0;
/// Max height for scrollable lists (Skills/Shop tabs, Bonds section), in points.
pub(super) const LIST_MAX_HEIGHT: f32 = 300.0;

/// The overlay ID used during registration — must match for show/hide calls.
pub(super) const OVERLAY_ID: &str = "training_tracker_overlay";
