use crate::manifest::FileEntry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;
use twox_hash::XxHash64;
use std::hash::Hasher;

const CACHE_PATH: &str = "lethe-file-cache.json";

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct FileCache {
    pub entries: HashMap<String, CachedFileInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedFileInfo {
    pub last_modified: u64,
    pub xxhash: String,
}

#[derive(Debug, PartialEq)]
pub enum CheckResult {
    Valid,
    Missing,
    Mismatched,
}

/// Compute XXHash64 of a file. Must match C# System.IO.Hashing.XxHash64 (seed=0).
/// Returns 16-character lowercase hex string.
pub fn compute_xxhash64(file_path: &str) -> Result<String, String> {
    let mut file = fs::File::open(file_path).map_err(|e| format!("Cannot open {}: {}", file_path, e))?;
    let mut hasher = XxHash64::with_seed(0);
    let mut buffer = [0u8; 64 * 1024]; // 64KB chunks, matching C#
    loop {
        let bytes_read = file.read(&mut buffer).map_err(|e| format!("Read error: {}", e))?;
        if bytes_read == 0 {
            break;
        }
        hasher.write(&buffer[..bytes_read]);
    }
    let hash = hasher.finish();
    Ok(format!("{:016x}", hash))
}

pub fn load_cache() -> FileCache {
    if let Ok(contents) = fs::read_to_string(CACHE_PATH) {
        if let Ok(cache) = serde_json::from_str(&contents) {
            return cache;
        }
    }
    FileCache::default()
}

pub fn save_cache(cache: &FileCache) {
    if let Ok(json) = serde_json::to_string_pretty(cache) {
        let _ = fs::write(CACHE_PATH, json);
    }
}

/// Check a single file against the manifest entry.
/// Uses timestamp caching to avoid re-hashing unchanged files.
pub fn check_file(entry: &FileEntry, cache: &mut FileCache) -> CheckResult {
    let file_path = &entry.path;

    // File doesn't exist on disk
    if !Path::new(file_path).exists() {
        return CheckResult::Missing;
    }

    // Quick size check
    let metadata = match fs::metadata(file_path) {
        Ok(m) => m,
        Err(_) => return CheckResult::Missing,
    };

    if metadata.len() != entry.size {
        return CheckResult::Mismatched;
    }

    // Get last modified timestamp
    let last_modified = match metadata.modified() {
        Ok(time) => time
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        Err(_) => 0,
    };

    // Check cache - if file hasn't been modified, trust cached hash
    if let Some(cached) = cache.entries.get(file_path) {
        if cached.last_modified == last_modified {
            return if cached.xxhash == entry.xxhash {
                CheckResult::Valid
            } else {
                CheckResult::Mismatched
            };
        }
    }

    // Cache miss or file modified - compute hash
    match compute_xxhash64(file_path) {
        Ok(hash) => {
            // Update cache
            cache.entries.insert(
                file_path.clone(),
                CachedFileInfo {
                    last_modified,
                    xxhash: hash.clone(),
                },
            );

            if hash == entry.xxhash {
                CheckResult::Valid
            } else {
                CheckResult::Mismatched
            }
        }
        Err(_) => CheckResult::Mismatched,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_xxhash64_known_value() {
        // Create a temporary file with known content
        let path = "test-hash-file.bin";
        let data = b"Hello, Lethe Launcher! This is a test file for XXHash64 verification.";
        let mut file = fs::File::create(path).unwrap();
        file.write_all(data).unwrap();

        let hash = compute_xxhash64(path).unwrap();
        // Just verify it's 16 hex chars and doesn't panic
        assert_eq!(hash.len(), 16);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));

        // Same file should produce same hash
        let hash2 = compute_xxhash64(path).unwrap();
        assert_eq!(hash, hash2);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_cache_save_load_roundtrip() {
        let path = "test-cache.json";
        let mut cache = FileCache::default();
        cache.entries.insert(
            "test/file.txt".to_string(),
            CachedFileInfo {
                last_modified: 1234567890,
                xxhash: "abcdef0123456789".to_string(),
            },
        );

        // Use a custom path for test
        let json = serde_json::to_string_pretty(&cache).unwrap();
        fs::write(path, &json).unwrap();

        let contents = fs::read_to_string(path).unwrap();
        let loaded: FileCache = serde_json::from_str(&contents).unwrap();
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(
            loaded.entries.get("test/file.txt").unwrap().xxhash,
            "abcdef0123456789"
        );

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_check_file_missing() {
        let entry = FileEntry {
            path: "nonexistent_file_xyz.bin".to_string(),
            size: 100,
            xxhash: "abc".to_string(),
        };
        let mut cache = FileCache::default();
        assert_eq!(check_file(&entry, &mut cache), CheckResult::Missing);
    }

    #[test]
    fn test_check_file_size_mismatch() {
        let path = "test-size-mismatch.bin";
        fs::write(path, b"short").unwrap();
        let entry = FileEntry { path: path.to_string(), size: 99999, xxhash: "abc".into() };
        let mut cache = FileCache::default();
        assert_eq!(check_file(&entry, &mut cache), CheckResult::Mismatched);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_check_file_hash_mismatch() {
        let path = "test-hash-mismatch.bin";
        let data = b"some test data for hashing";
        fs::write(path, data).unwrap();
        let entry = FileEntry { path: path.to_string(), size: data.len() as u64, xxhash: "0000000000000000".into() };
        let mut cache = FileCache::default();
        assert_eq!(check_file(&entry, &mut cache), CheckResult::Mismatched);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_check_file_valid_and_cached() {
        let path = "test-valid.bin";
        let data = b"valid test file content";
        fs::write(path, data).unwrap();
        let correct_hash = compute_xxhash64(path).unwrap();
        let entry = FileEntry { path: path.to_string(), size: data.len() as u64, xxhash: correct_hash };

        let mut cache = FileCache::default();
        assert_eq!(check_file(&entry, &mut cache), CheckResult::Valid);
        assert!(cache.entries.contains_key(path));

        // Second check should hit cache
        let mut cache2 = cache.clone();
        assert_eq!(check_file(&entry, &mut cache2), CheckResult::Valid);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_xxhash64_empty_file() {
        let path = "test-empty.bin";
        fs::File::create(path).unwrap();
        let hash = compute_xxhash64(path).unwrap();
        assert_eq!(hash.len(), 16);
        let hash2 = compute_xxhash64(path).unwrap();
        assert_eq!(hash, hash2);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_xxhash64_error_on_nonexistent() {
        assert!(compute_xxhash64("definitely_does_not_exist_12345.bin").is_err());
    }
}
