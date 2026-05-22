use std::path::{Path, PathBuf};

const PATHS_JSON: &str = include_str!("../../../config/openwhisper-paths.json");

#[derive(Debug, serde::Deserialize)]
struct PathsManifest {
    #[serde(rename = "windowsMsvcTarget")]
    windows_msvc_target: String,
    #[serde(rename = "debugTargetSubdirs")]
    debug_target_subdirs: Vec<Vec<String>>,
    executables: std::collections::HashMap<String, String>,
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

pub fn executable_name(crate_key: &str) -> Option<&'static str> {
    manifest()
        .executables
        .get(crate_key)
        .map(String::as_str)
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
        assert_eq!(
            executable_name("openwhisper-native"),
            Some("openwhisper-native.exe")
        );
        assert_eq!(executable_name("openwhisper-asr-rs"), Some("ow-asr-rs.exe"));
    }
}
