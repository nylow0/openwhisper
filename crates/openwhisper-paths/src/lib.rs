use std::path::{Path, PathBuf};

const PATHS_JSON: &str = include_str!("../../../config/openwhisper-paths.json");

#[derive(Debug, serde::Deserialize)]
struct PathsManifest {
    #[serde(rename = "windowsMsvcTarget")]
    windows_msvc_target: String,
    #[serde(rename = "linuxGnuTarget")]
    linux_gnu_target: String,
    #[serde(rename = "debugTargetSubdirs")]
    debug_target_subdirs: Vec<Vec<String>>,
    executables: std::collections::HashMap<String, String>,
    executables_linux: std::collections::HashMap<String, String>,
}

fn manifest() -> &'static PathsManifest {
    static MANIFEST: std::sync::OnceLock<PathsManifest> = std::sync::OnceLock::new();
    MANIFEST.get_or_init(|| {
        serde_json::from_str(PATHS_JSON).expect("config/openwhisper-paths.json must be valid")
    })
}

pub fn windows_msvc_target() -> &'static str {
    &manifest().windows_msvc_target
}

pub fn linux_gnu_target() -> &'static str {
    &manifest().linux_gnu_target
}

pub fn native_target() -> &'static str {
    if cfg!(windows) {
        windows_msvc_target()
    } else {
        linux_gnu_target()
    }
}

pub fn executable_name(crate_key: &str) -> Option<&'static str> {
    let map = if cfg!(windows) {
        &manifest().executables
    } else {
        &manifest().executables_linux
    };
    map.get(crate_key).map(String::as_str)
}

pub fn debug_executable_candidates(crate_root: &Path, crate_key: &str) -> Vec<PathBuf> {
    let name = match executable_name(crate_key) {
        Some(name) => name,
        None => return Vec::new(),
    };
    manifest()
        .debug_target_subdirs
        .iter()
        .map(|segments| {
            let mut path = crate_root.to_path_buf();
            for segment in segments {
                path.push(segment);
            }
            path.push(name);
            path
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_lists_native_and_asr_executables() {
        if cfg!(windows) {
            assert_eq!(
                executable_name("openwhisper-native"),
                Some("openwhisper-native.exe")
            );
            assert_eq!(executable_name("openwhisper-asr-rs"), Some("ow-asr-rs.exe"));
        } else {
            assert_eq!(
                executable_name("openwhisper-native"),
                Some("openwhisper-native")
            );
            assert_eq!(executable_name("openwhisper-asr-rs"), Some("ow-asr-rs"));
        }
    }

    #[test]
    fn native_target_matches_platform() {
        if cfg!(windows) {
            assert_eq!(native_target(), "x86_64-pc-windows-msvc");
        } else {
            assert_eq!(native_target(), "x86_64-unknown-linux-gnu");
        }
    }
}
