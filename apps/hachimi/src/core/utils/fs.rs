//! Filesystem and path helpers: JSON writing, path joining, and game data paths.

use std::{io::Write, path::Path, time::SystemTime};

use serde::Serialize;

use crate::core::{Error, Hachimi};

pub fn concat_unix_path(left: &str, right: &str) -> String {
    let mut str = String::with_capacity(left.len() + 1 + right.len());
    str.push_str(left);
    str.push('/');
    str.push_str(right);
    str
}

pub fn write_json_file<T: Serialize, P: AsRef<Path>>(data: &T, path: P) -> Result<(), Error> {
    let file = std::fs::File::create(path)?;
    let mut writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, data)?;
    writer.flush()?;
    Ok(())
}

pub fn get_file_modified_time<P: AsRef<Path>>(path: P) -> Option<SystemTime> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    metadata.modified().ok()
}

pub fn get_data_path() -> String {
    {
        use crate::{
            core::game::Region, il2cpp::ext::Il2CppStringExt, il2cpp::hook::UnityEngine_CoreModule::Application,
            windows::utils::get_game_dir,
        };

        let game = &Hachimi::instance().game;
        let jp_steam_data_path = get_game_dir().join("UmamusumePrettyDerby_Jpn_Data").join("Persistent");
        let new_jp_dmm_data_path = get_game_dir().join("umamusume_Data").join("Persistent");

        let dir_ok = |path: &std::path::Path| {
            path.exists()
                && std::fs::read_dir(path).is_ok_and(|mut d| d.next().is_some())
                && path.join("master").join("master.mdb").exists()
        };

        if game.region == Region::Japan && game.is_steam_release && dir_ok(&jp_steam_data_path) {
            jp_steam_data_path.to_string_lossy().to_string()
        } else if game.region == Region::Japan && !game.is_steam_release && dir_ok(&new_jp_dmm_data_path) {
            new_jp_dmm_data_path.to_string_lossy().to_string()
        } else {
            // SAFETY: FFI / raw pointer operation required by IL2CPP interop
            unsafe { (*Application::get_persistentDataPath()).as_utf16str() }.to_string()
        }
    }
}

pub fn get_masterdb_path() -> String {
    info!("get_masterdb_path base: {}", get_data_path());
    format!("{}/master/master.mdb", get_data_path())
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[test]
    fn concat_unix_path_basic() {
        assert_eq!(concat_unix_path("a", "b"), "a/b");
        assert_eq!(concat_unix_path("/foo", "bar.txt"), "/foo/bar.txt");
    }
}
