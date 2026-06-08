use serde::{Deserialize, Serialize};

const UPDATE_URL: &str =
    "https://github.com/LEAGUE-OF-NINE/LetheLauncher/releases/latest/download/latest.json";

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: String,
    pub download_url: String,
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

/// Download and apply an update: downloads the new exe, saves it as LimbusCompany.exe.new,
/// then returns. The launcher should exit and a restart mechanism should swap files.
pub async fn download_update(update: &UpdateInfo) -> Result<(), String> {
    crate::lethe_log!("Downloading update from {}", update.download_url);

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

    // Save as .new alongside the current executable
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Cannot get current exe path: {}", e))?;
    let new_exe = current_exe.with_extension("exe.new");

    std::fs::write(&new_exe, &bytes)
        .map_err(|e| format!("Failed to write update: {}", e))?;

    crate::lethe_log!(
        "Update downloaded to {}",
        new_exe.display()
    );

    // Write a small batch script to replace old exe with new one and restart
    let script_path = current_exe
        .with_file_name("update.bat");
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

    Ok(())
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
    fn test_version_with_suffix() {
        assert!(is_newer("0.2.0-alpha", "0.1.0"));
        assert!(is_newer("0.1.0-beta.2", "0.1.0-beta.1"));
    }
}
