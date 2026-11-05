use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct ModInfo {
    pub name: String,
    pub enabled: bool,
}

const MODS_REL_PATH: &str = "BepInEx/plugins/Lethe/mods";

/// List all mods in the Lethe mods folder. A folder prefixed with `DISABLED_`
/// or `FULLDISABLED_` is a disabled mod.
pub fn list_mods() -> Vec<ModInfo> {
    list_mods_in(Path::new(MODS_REL_PATH))
}

pub fn toggle_mod(name: &str) -> Result<ModInfo, String> {
    toggle_mod_in(Path::new(MODS_REL_PATH), name)
}

fn list_mods_in(mods_path: &Path) -> Vec<ModInfo> {
    let Ok(entries) = std::fs::read_dir(mods_path) else {
        crate::lethe_log!("Mods directory not found at {}", mods_path.display());
        return vec![];
    };

    let mut mods: Vec<ModInfo> = entries
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if let Some(stripped) = name.strip_prefix("DISABLED_") {
                ModInfo { name: stripped.to_string(), enabled: false }
            } else if let Some(stripped) = name.strip_prefix("FULLDISABLED_") {
                ModInfo { name: stripped.to_string(), enabled: false }
            } else {
                ModInfo { name, enabled: true }
            }
        })
        .collect();
    mods.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    crate::lethe_log!("Found {} mod(s)", mods.len());
    mods
}

/// Toggle a mod on/off by renaming its folder.
fn toggle_mod_in(mods_path: &Path, name: &str) -> Result<ModInfo, String> {
    let enabled_path = mods_path.join(name);
    let disabled_path: PathBuf = mods_path.join(format!("DISABLED_{}", name));
    let fulldisabled_path = mods_path.join(format!("FULLDISABLED_{}", name));

    let (from, to, enabled) = if enabled_path.exists() {
        (&enabled_path, &disabled_path, false)
    } else if disabled_path.exists() {
        (&disabled_path, &enabled_path, true)
    } else if fulldisabled_path.exists() {
        (&fulldisabled_path, &enabled_path, true)
    } else {
        return Err(format!("Mod '{}' not found", name));
    };

    std::fs::rename(from, to).map_err(|e| {
        let verb = if enabled { "enable" } else { "disable" };
        format!("Failed to {} mod: {}", verb, e)
    })?;
    Ok(ModInfo { name: name.to_string(), enabled })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_mods_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lethe-mods-test-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn lists_enabled_and_disabled_mods_sorted() {
        let dir = temp_mods_dir("list");
        std::fs::create_dir(dir.join("Zeta")).unwrap();
        std::fs::create_dir(dir.join("DISABLED_alpha")).unwrap();
        std::fs::create_dir(dir.join("FULLDISABLED_Beta")).unwrap();
        std::fs::write(dir.join("not-a-mod.txt"), "x").unwrap();

        let mods = list_mods_in(&dir);
        assert_eq!(
            mods,
            vec![
                ModInfo { name: "alpha".into(), enabled: false },
                ModInfo { name: "Beta".into(), enabled: false },
                ModInfo { name: "Zeta".into(), enabled: true },
            ]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn toggle_round_trips_folder_names() {
        let dir = temp_mods_dir("toggle");
        std::fs::create_dir(dir.join("Cool")).unwrap();
        std::fs::create_dir(dir.join("FULLDISABLED_Old")).unwrap();

        assert!(!toggle_mod_in(&dir, "Cool").unwrap().enabled);
        assert!(dir.join("DISABLED_Cool").exists());
        assert!(toggle_mod_in(&dir, "Cool").unwrap().enabled);
        assert!(dir.join("Cool").exists());

        assert!(toggle_mod_in(&dir, "Old").unwrap().enabled);
        assert!(dir.join("Old").exists());

        assert!(toggle_mod_in(&dir, "Missing").is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn missing_dir_lists_nothing() {
        let dir = std::env::temp_dir().join("lethe-mods-test-definitely-missing");
        assert!(list_mods_in(&dir).is_empty());
    }
}
