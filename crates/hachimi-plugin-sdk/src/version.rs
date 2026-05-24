//! API version feature gates matching host slot introductions.

use hachimi_plugin_abi::API_VERSION;

/// Host plugin API version supplied to `hachimi_init`.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ApiVersion(i32);

impl ApiVersion {
    #[must_use]
    pub const fn new(version: i32) -> Self {
        Self(version)
    }

    #[must_use]
    pub const fn raw(self) -> i32 {
        self.0
    }

    #[must_use]
    pub const fn supports_overlay(self) -> bool {
        self.0 >= 3
    }

    #[must_use]
    pub const fn supports_min_width(self) -> bool {
        self.0 >= 4
    }

    #[must_use]
    pub const fn supports_overlay_visibility(self) -> bool {
        self.0 >= 5
    }

    #[must_use]
    pub const fn supports_collapsing(self) -> bool {
        self.0 >= 6
    }

    #[must_use]
    pub const fn supports_font_size(self) -> bool {
        self.0 >= 7
    }

    #[must_use]
    pub const fn current_host() -> Self {
        Self(API_VERSION)
    }
}
