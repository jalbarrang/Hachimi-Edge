use std::path::{Path, PathBuf};

use crate::core::game::Region;

use super::utils;

pub fn get_package_name() -> String {
    utils::get_exec_path().to_str().expect("valid UTF-8").to_owned()
}

pub fn get_region(package_name: &str) -> Region {
    match Path::new(package_name)
        .file_name()
        .expect("unexpected failure")
        .to_str()
        .expect("unexpected failure")
        .to_ascii_lowercase()
        .as_str()
    {
        "umamusume.exe" | "umamusumeprettyderby_jpn.exe" => Region::Japan,
        "umamusumeprettyderby.exe" => Region::Global,
        _ => Region::Unknown,
    }
}

pub fn get_data_dir(package_name: &str) -> PathBuf {
    Path::new(package_name)
        .parent()
        .expect("unexpected failure")
        .join("hachimi")
}

pub fn is_steam_release(package_name: &str) -> bool {
    let exec_path = Path::new(package_name);
    let is_jp_steam = exec_path
        .file_stem()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("umamusumeprettyderby_jpn"));
    is_jp_steam
}
