use crate::manifest::FileEntry;
use crate::verifier::compute_xxhash64;
use std::fs;
use std::path::Path;

/// Get the local Steam game folder path for Limbus Company.
#[cfg(windows)]
pub fn get_steam_game_path() -> String {
    use winreg::enums::*;
    use winreg::RegKey;

    if let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"SOFTWARE\WOW6432Node\Valve\Steam")
    {
        if let Ok(steam_path) = key.get_value::<String, _>("InstallPath") {
            return format!(
                "{}/steamapps/common/Limbus Company",
                steam_path.trim_end_matches('\\')
            );
        }
    }

    // Fallback
    "C:/Program Files (x86)/Steam/steamapps/common/Limbus Company".to_string()
}

#[cfg(not(windows))]
pub fn get_steam_game_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
    format!(
        "{}/Library/Application Support/CrossOver/Bottles/Steam/drive_c/Program Files (x86)/Steam/steamapps/common/Limbus Company",
        home
    )
}

/// Try to copy a file from the local Steam installation if it matches the expected hash.
/// Returns true if the file was successfully copied from Steam.
pub async fn try_copy_from_steam(entry: &FileEntry) -> Result<bool, String> {
    let steam_path = get_steam_game_path();
    let local_path = format!("{}/{}", steam_path, entry.path);

    if !Path::new(&local_path).exists() {
        return Ok(false);
    }

    // Check size
    let metadata = fs::metadata(&local_path)
        .map_err(|e| format!("Cannot read metadata: {}", e))?;
    if metadata.len() != entry.size {
        crate::lethe_log!("Size mismatch for local copy {}: expected {}, got {}",
            entry.path, entry.size, metadata.len());
        return Ok(false);
    }

    // Check hash
    let hash = compute_xxhash64(&local_path)?;
    if hash != entry.xxhash {
        crate::lethe_log!("Hash mismatch for local copy {}: expected {}, got {}",
            entry.path, entry.xxhash, hash);
        return Ok(false);
    }

    // Copy to target
    if let Some(parent) = Path::new(&entry.path).parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create directory: {}", e))?;
    }

    fs::copy(&local_path, &entry.path)
        .map_err(|e| format!("Cannot copy {}: {}", entry.path, e))?;

    crate::lethe_log!("Copied from local: {} ({} bytes)", entry.path, entry.size);
    Ok(true)
}
