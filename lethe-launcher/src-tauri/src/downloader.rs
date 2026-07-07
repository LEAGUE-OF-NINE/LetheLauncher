use crate::manifest::{FileEntry, FileManifest};
use crate::steam::try_copy_from_steam;
use futures_util::StreamExt;
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::Path;
use tauri::{AppHandle, Emitter};

const MANIFEST_URL: &str = "https://files.lethelc.site/lethe-manifest.json";
const DOWNLOAD_BASE_URL: &str = "https://files.lethelc.site/download/";

const ADDITIONAL_DLLS: &[(&str, &str)] = &[
    ("https://api.lethelc.site/Lethe.dll", "Lethe.dll"),
    (
        "https://api.lethelc.site/ModularSkillScripts.dll",
        "ModularSkillScripts.dll",
    ),
    ("https://api.lethelc.site/motions.dll", "motions.dll")
];

#[derive(Debug, Serialize, Clone)]
pub struct StatusChangeEvent {
    pub message: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct DownloadProgressEvent {
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub percent: f64,
    pub current_file: String,
}

pub async fn download_manifest(app_handle: &AppHandle) -> Result<FileManifest, String> {
    let _ = app_handle.emit(
        "status-change",
        StatusChangeEvent {
            message: "Downloading manifest...".to_string(),
        },
    );

    let response = reqwest::get(MANIFEST_URL)
        .await
        .map_err(|e| format!("Failed to download manifest: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("Manifest server returned HTTP {}", status.as_u16()));
    }

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read manifest response: {}", e))?;

    let manifest: FileManifest =
        serde_json::from_str(&body).map_err(|e| format!("Failed to parse manifest JSON: {}", e))?;

    crate::lethe_log!(
        "Downloaded manifest: {} files, {} bytes",
        manifest.total_files,
        manifest.total_size
    );

    Ok(manifest)
}

/// Download a single file from the remote server with progress reporting.
pub async fn download_file(
    app_handle: &AppHandle,
    entry: &FileEntry,
    base_downloaded: u64,
    total_download_bytes: u64,
) -> Result<(), String> {
    let url = format!("{}{}", DOWNLOAD_BASE_URL, entry.path);

    let _ = app_handle.emit(
        "status-change",
        StatusChangeEvent {
            message: format!("Downloading {}...", entry.path),
        },
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to download {}: {}", entry.path, e))?;

    if !response.status().is_success() {
        return Err(format!(
            "HTTP {} for {}",
            response.status().as_u16(),
            entry.path
        ));
    }

    // Ensure directory exists
    if let Some(parent) = Path::new(&entry.path).parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create directory for {}: {}", entry.path, e))?;
    }

    let mut file = fs::File::create(&entry.path)
        .map_err(|e| format!("Cannot create file {}: {}", entry.path, e))?;

    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download error for {}: {}", entry.path, e))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Write error for {}: {}", entry.path, e))?;
        downloaded += chunk.len() as u64;

        let current_total = base_downloaded + downloaded;
        let percent = if total_download_bytes > 0 {
            (current_total as f64 / total_download_bytes as f64) * 100.0
        } else {
            0.0
        };

        let _ = app_handle.emit(
            "download-progress",
            DownloadProgressEvent {
                bytes_downloaded: current_total,
                total_bytes: total_download_bytes,
                percent,
                current_file: entry.path.clone(),
            },
        );
    }

    crate::lethe_log!("Downloaded: {} ({} bytes)", entry.path, entry.size);
    Ok(())
}

/// Download files sequentially (downloads are network-bound, not worth parallelizing).
pub async fn download_files(
    app_handle: &AppHandle,
    files: &[FileEntry],
    base_bytes: u64,
    total_bytes: u64,
) -> Result<u64, String> {
    let mut processed = base_bytes;

    for entry in files {
        // Try local Steam copy first
        let copied = try_copy_from_steam(entry).await?;
        if copied {
            processed += entry.size;
            let percent = if total_bytes > 0 {
                (processed as f64 / total_bytes as f64) * 100.0
            } else {
                0.0
            };
            let _ = app_handle.emit(
                "download-progress",
                DownloadProgressEvent {
                    bytes_downloaded: processed,
                    total_bytes,
                    percent,
                    current_file: entry.path.clone(),
                },
            );
            continue;
        }

        // Download from remote
        download_file(app_handle, entry, processed, total_bytes).await?;
        processed += entry.size;
    }

    Ok(processed)
}

/// Download additional DLLs (Lethe.dll, ModularSkillScripts.dll) to BepInEx/plugins/
pub async fn download_additional_dlls(app_handle: &AppHandle) -> Result<(), String> {
    let plugins_dir = Path::new("BepInEx").join("plugins");
    fs::create_dir_all(&plugins_dir)
        .map_err(|e| format!("Cannot create BepInEx/plugins: {}", e))?;

    for (url, filename) in ADDITIONAL_DLLS {
        let _ = app_handle.emit(
            "status-change",
            StatusChangeEvent {
                message: format!("Downloading {}...", filename),
            },
        );

        let response = reqwest::get(*url)
            .await
            .map_err(|e| format!("Failed to download {}: {}", filename, e))?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("Failed to read {}: {}", filename, e))?;

        let file_path = plugins_dir.join(filename);
        fs::write(&file_path, &bytes)
            .map_err(|e| format!("Cannot write {}: {}", filename, e))?;

        crate::lethe_log!(
            "Downloaded additional DLL: {} ({} bytes)",
            filename,
            bytes.len()
        );
    }

    Ok(())
}

#[allow(dead_code)]
pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::FileManifest;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
        assert_eq!(format_bytes(1073741824), "1.0 GB");
        assert_eq!(format_bytes(2684354560), "2.5 GB");
    }

    #[test]
    fn test_manifest_deserialization() {
        let json = r#"{
            "scanned_folder": "/tmp/test",
            "total_files": 3,
            "total_size": 12345,
            "files": [
                {"path": "foo/bar.dll", "size": 1000, "xxhash": "abcdef0123456789"},
                {"path": "baz.dat", "size": 2000, "xxhash": "0000000000000000"},
                {"path": "sub/dir/file.txt", "size": 9345, "xxhash": "ffffffffffffffff"}
            ]
        }"#;

        let manifest: FileManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.scanned_folder, "/tmp/test");
        assert_eq!(manifest.total_files, 3);
        assert_eq!(manifest.total_size, 12345);
        assert_eq!(manifest.files.len(), 3);
        assert_eq!(manifest.files[0].path, "foo/bar.dll");
        assert_eq!(manifest.files[2].path, "sub/dir/file.txt");
    }

    #[test]
    fn test_manifest_empty_files() {
        let json = r#"{"scanned_folder":"/","total_files":0,"total_size":0,"files":[]}"#;
        let manifest: FileManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.files.len(), 0);
        assert_eq!(manifest.total_files, 0);
    }
}
