use serde::{Deserialize, Serialize};

const UPDATE_URL: &str =
    "https://github.com/LEAGUE-OF-NINE/LetheLauncher/releases/latest/download/latest.json";

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: String,
    pub download_url: String,
    /// Lowercase hex SHA-256 of the exe at download_url. Verified before applying.
    #[serde(default)]
    pub sha256: String,
}

/// Check for updates by fetching the latest.json from GitHub releases.
/// Returns Some(UpdateInfo) if a newer version is available.
pub async fn check_for_updates(current_version: &str) -> Result<Option<UpdateInfo>, String> {
    let response = reqwest::get(UPDATE_URL)
        .await
        .map_err(|e| format!("Failed to check for updates: {}", e))?;

    if !response.status().is_success() {
        crate::lethe_log!(
            "Update check returned HTTP {} (no update available or endpoint missing)",
            response.status().as_u16()
        );
        return Ok(None);
    }

    let update: UpdateInfo = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse update info: {}", e))?;

    // Simple semver-ish comparison: compare version strings
    if is_newer(&update.version, current_version) {
        crate::lethe_log!(
            "Update available: {} -> {}",
            current_version,
            update.version
        );
        Ok(Some(update))
    } else {
        crate::lethe_log!("Already on latest version: {}", current_version);
        Ok(None)
    }
}

/// Compare two version strings. Returns true if `candidate` is greater than `current`.
fn is_newer(candidate: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse::<u32>().ok())
            .collect()
    };

    let a = parse(candidate);
    let b = parse(current);

    for (ac, bc) in a.iter().zip(b.iter()) {
        if ac > bc {
            return true;
        }
        if ac < bc {
            return false;
        }
    }
    // If all common components equal, the one with more components is newer
    // (e.g. 0.1.1 > 0.1)
    a.len() > b.len()
}

/// Compute lowercase hex SHA-256 of a byte slice.
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Download, verify, and apply an update. Downloads the new exe, checks its SHA-256
/// against the manifest, writes it as LimbusCompany.exe.new, spawns a detached script
/// that swaps in the new exe and restarts, then exits this process so the swap can proceed.
/// On success this function does NOT return (the process is terminated).
pub async fn download_update(update: &UpdateInfo) -> Result<(), String> {
    crate::lethe_log!("Downloading update from {}", update.download_url);

    // Refuse to apply an update we cannot verify. Older manifests without a checksum
    // (or a tampered one that dropped it) must be installed manually.
    if update.sha256.trim().is_empty() {
        return Err(
            "Update manifest is missing a SHA-256 checksum; refusing to auto-apply. \
             Please update manually from the releases page."
                .to_string(),
        );
    }

    let response = reqwest::get(&update.download_url)
        .await
        .map_err(|e| format!("Failed to download update: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Update download failed: HTTP {}",
            response.status().as_u16()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read update: {}", e))?;

    // Verify integrity BEFORE writing anything to disk.
    let actual = sha256_hex(&bytes);
    if !actual.eq_ignore_ascii_case(update.sha256.trim()) {
        return Err(format!(
            "Update checksum mismatch: expected {}, got {}. Aborting.",
            update.sha256.trim(),
            actual
        ));
    }
    crate::lethe_log!("Update checksum verified ({} bytes)", bytes.len());

    // Save as .new alongside the current executable
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Cannot get current exe path: {}", e))?;
    let new_exe = current_exe.with_extension("exe.new");

    std::fs::write(&new_exe, &bytes)
        .map_err(|e| format!("Failed to write update: {}", e))?;

    crate::lethe_log!("Update downloaded to {}", new_exe.display());

    // Write a small batch script to replace old exe with new one and restart.
    // The `timeout` gives this process time to exit and release the exe lock before `move`.
    let script_path = current_exe.with_file_name("update.bat");
    let script = format!(
        "@echo off\r\n\
         timeout /t 2 /nobreak >nul\r\n\
         move /Y \"{}\" \"{}\"\r\n\
         start \"\" \"{}\"\r\n\
         del \"%~f0\"\r\n",
        new_exe.display(),
        current_exe.display(),
        current_exe.display()
    );
    std::fs::write(&script_path, script)
        .map_err(|e| format!("Failed to write update script: {}", e))?;

    crate::lethe_log!("Update script written to {}", script_path.display());

    // Launch the swap script detached, then exit so it can overwrite our (locked) exe.
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args([
                "/C",
                "start",
                "",
                "/min",
                &script_path.to_string_lossy(),
            ])
            .spawn()
            .map_err(|e| format!("Failed to launch update script: {}", e))?;
        crate::lethe_log!("Update script launched; exiting to apply update");
        std::process::exit(0);
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Non-Windows launcher is not shipped; nothing swaps the exe here.
        Err("Auto-update is only supported on Windows.".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_newer() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("0.1.1", "0.1.0"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(is_newer("0.10.0", "0.9.0"));
        assert!(is_newer("2.0.0", "1.99.99"));
        assert!(is_newer("0.0.2", "0.0.1"));
        assert!(is_newer("1.2.3", "1.2.2"));
        assert!(is_newer("v1.2.3", "v1.2.2"));
        assert!(is_newer("0.2.0-beta", "0.1.0"));
    }

    #[test]
    fn test_version_not_newer() {
        assert!(!is_newer("0.0.9", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        assert!(!is_newer("1.0.0", "2.0.0"));
        assert!(!is_newer("0.1.0", "1.0.0"));
        assert!(!is_newer("0.0.1", "0.0.2"));
    }

    #[test]
    fn test_version_same() {
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("0.1.5", "0.1.5"));
        assert!(!is_newer("v0.1.0", "0.1.0"));
    }

    #[test]
    fn test_sha256_hex_known_values() {
        // Known SHA-256 vectors; the updater rejects the download unless this matches.
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn test_version_with_suffix() {
        assert!(is_newer("0.2.0-alpha", "0.1.0"));
        assert!(is_newer("0.1.0-beta.2", "0.1.0-beta.1"));
    }
}
