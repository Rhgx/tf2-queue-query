use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

#[cfg(target_os = "windows")]
use winreg::{RegKey, enums::HKEY_CURRENT_USER};

/// Steam install folders to search, most likely first.
#[cfg(target_os = "windows")]
fn steam_roots() -> Vec<PathBuf> {
    let registry = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Valve\Steam")
        .and_then(|steam| steam.get_value::<String, _>("SteamPath"))
        .ok()
        .map(PathBuf::from);
    registry
        .into_iter()
        .chain([PathBuf::from(r"C:\Program Files (x86)\Steam")])
        .collect()
}

/// Steam install folders to search, most likely first. Flatpak and Snap installs are not
/// searched: their sandbox is likely to keep the Steam API from reaching the client.
#[cfg(target_os = "linux")]
fn steam_roots() -> Vec<PathBuf> {
    std::env::var_os("HOME").map_or_else(Vec::new, |home| {
        let home = PathBuf::from(home);
        vec![home.join(".local/share/Steam"), home.join(".steam/steam")]
    })
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
    for steam_root in steam_roots() {
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
pub fn steam_api_library(tf2_root: &Path) -> PathBuf {
    tf2_root.join("bin").join("x64").join("steam_api64.dll")
}

#[cfg(target_os = "linux")]
pub fn steam_api_library(tf2_root: &Path) -> PathBuf {
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
