use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

#[cfg(target_os = "windows")]
use winreg::{RegKey, enums::HKEY_CURRENT_USER};

#[cfg(target_os = "windows")]
fn steam_root_from_registry() -> Option<PathBuf> {
    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    let steam = current_user.open_subkey(r"Software\Valve\Steam").ok()?;
    steam
        .get_value::<String, _>("SteamPath")
        .ok()
        .map(PathBuf::from)
}

#[cfg(target_os = "linux")]
fn steam_root_from_filesystem() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    [
        PathBuf::from(&home).join(".local/share/Steam"),
        PathBuf::from(&home).join(".steam/steam"),
    ]
    .into_iter()
    .find(|root| root.join("steamapps").join("libraryfolders.vdf").is_file())
}

fn quoted_values(text: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        if character != '"' {
            continue;
        }
        let mut value = String::new();
        while let Some(character) = characters.next() {
            if character == '"' {
                break;
            }
            if character == '\\' {
                if let Some(next) = characters.next() {
                    value.push(next);
                }
            } else {
                value.push(character);
            }
        }
        values.push(value);
    }
    values
}

fn library_roots(steam_root: &Path) -> Vec<PathBuf> {
    let mut roots = vec![steam_root.to_path_buf()];
    let library_file = steam_root.join("steamapps").join("libraryfolders.vdf");
    let Ok(text) = fs::read_to_string(library_file) else {
        return roots;
    };
    let values = quoted_values(&text);
    for pair in values.windows(2) {
        if pair[0].eq_ignore_ascii_case("path") {
            let candidate = PathBuf::from(&pair[1]);
            if !roots.iter().any(|root| root == &candidate) {
                roots.push(candidate);
            }
        }
    }
    roots
}

pub fn find_tf2_root(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return validate_tf2_root(&path);
    }
    #[cfg(target_os = "windows")]
    let steam_root = steam_root_from_registry();
    #[cfg(target_os = "linux")]
    let steam_root = steam_root_from_filesystem();
    if let Some(steam_root) = steam_root {
        for library in library_roots(&steam_root) {
            let candidate = library
                .join("steamapps")
                .join("common")
                .join("Team Fortress 2");
            if candidate.join("tf").join("steam.inf").is_file() {
                return validate_tf2_root(&candidate);
            }
        }
    }
    #[cfg(target_os = "windows")]
    let fallback = PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps\common\Team Fortress 2");
    #[cfg(target_os = "linux")]
    let fallback = std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(".local/share/Steam/steamapps/common/Team Fortress 2"))
        .unwrap_or_default();
    if fallback.join("tf").join("steam.inf").is_file() {
        return validate_tf2_root(&fallback);
    }
    bail!("Team Fortress 2 was not found; pass --tf2-root <directory>")
}

fn validate_tf2_root(path: &Path) -> Result<PathBuf> {
    if !path.join("tf").join("steam.inf").is_file() {
        bail!(
            "{} is not a TF2 installation (tf/steam.inf is missing)",
            path.display()
        );
    }
    let steam_api = steam_api_library(path);
    if !steam_api.is_file() {
        bail!("{} is missing {}", path.display(), steam_api.display());
    }
    path.canonicalize()
        .with_context(|| format!("could not resolve {}", path.display()))
}

#[cfg(target_os = "windows")]
fn steam_api_library(tf2_root: &Path) -> PathBuf {
    tf2_root.join("bin").join("x64").join("steam_api64.dll")
}

#[cfg(target_os = "linux")]
fn steam_api_library(tf2_root: &Path) -> PathBuf {
    tf2_root.join("bin").join("linux64").join("libsteam_api.so")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_steam_library_paths() {
        let values = quoted_values(r#""path" "D:\\SteamLibrary" "label" """#);
        assert_eq!(values, ["path", r"D:\SteamLibrary", "label", ""]);
    }
}
