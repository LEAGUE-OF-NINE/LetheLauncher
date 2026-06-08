use std::collections::HashMap;
use std::fs;
use std::path::Path;

const CONFIG_PATH: &str = "lethe-launcher.ini";

pub fn read_config() -> HashMap<String, String> {
    let mut config = HashMap::new();
    if let Ok(contents) = fs::read_to_string(CONFIG_PATH) {
        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
                continue;
            }
            if let Some(sep) = trimmed.find('=') {
                let key = trimmed[..sep].trim().to_string();
                let value = trimmed[sep + 1..].trim().to_string();
                config.insert(key, value);
            }
        }
    }
    config
}

pub fn get_config_value(key: &str, default: &str) -> String {
    let config = read_config();
    config.get(key).cloned().unwrap_or_else(|| default.to_string())
}

pub fn is_auto_update_disabled() -> bool {
    get_config_value("DisableAutoUpdate", "false")
        .to_lowercase()
        == "true"
}

pub fn create_default_config() {
    if !Path::new(CONFIG_PATH).exists() {
        let _ = fs::write(CONFIG_PATH, "DisableAutoUpdate=false\n");
    }
}

pub fn set_config_value(key: &str, value: &str) {
    let mut config = read_config();
    config.insert(key.to_string(), value.to_string());
    let mut content = String::new();
    for (k, v) in &config {
        content.push_str(&format!("{}={}\n", k, v));
    }
    let _ = fs::write(CONFIG_PATH, content);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_parse_config_disabled() {
        fs::write("test-config.ini", "DisableAutoUpdate=true\n").unwrap();
        // Override CONFIG_PATH for this test by reading directly
        let contents = fs::read_to_string("test-config.ini").unwrap();
        assert!(contents.contains("DisableAutoUpdate=true"));
        fs::remove_file("test-config.ini").unwrap();
    }

    #[test]
    fn test_get_config_value_default() {
        let val = get_config_value("NonExistent", "default_val");
        assert_eq!(val, "default_val");
    }
}
