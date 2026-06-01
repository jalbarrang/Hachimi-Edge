//! Asset metadata loaded alongside localized data files.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct AssetInfo<T> {
    #[cfg(target_os = "windows")]
    #[serde(default)]
    windows: AssetMetadata,

    pub data: Option<T>,
}

// Can't derive(Default), see rust-lang/rust#26925
impl<T> Default for AssetInfo<T> {
    fn default() -> Self {
        Self {
            #[cfg(target_os = "windows")]
            windows: Default::default(),

            data: None,
        }
    }
}

impl<T> AssetInfo<T> {
    pub fn metadata(self) -> AssetMetadata {
        #[cfg(target_os = "windows")]
        return self.windows;
    }

    pub fn metadata_ref(&self) -> &AssetMetadata {
        #[cfg(target_os = "windows")]
        return &self.windows;
    }
}

#[derive(Deserialize, Clone, Default)]
pub struct AssetMetadata {
    pub bundle_name: Option<String>,
}
