use crate::backend::manifest::{FileEntry, FileManifest};
use crate::backend::steam::try_copy_from_steam;
use futures_util::StreamExt;
use std::fs;
use crate::backend::sync::{Reporter, SyncEvent};
use std::hash::Hasher;
use std::io::Write;
use std::path::Path;
use twox_hash::XxHash64;

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

pub async fn download_manifest(reporter: &Reporter) -> Result<FileManifest, String> {
    reporter.status("Downloading manifest...".to_string());

    let response = crate::backend::http()
        .get(MANIFEST_URL)
        .send()
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

/// Where a file is written while it downloads. It only replaces the real file
/// once it is complete and verified, so a cancelled or failed download never
/// leaves a truncated game file behind.
pub fn partial_path(path: &str) -> String {
    format!("{}.lethe-part", path)
}

/// Move a finished temporary file over the real one.
fn replace_with(partial: &str, target: &str) -> Result<(), String> {
    fs::rename(partial, target).map_err(|e| {
        let _ = fs::remove_file(partial);
        format!("Cannot replace {}: {}", target, e)
    })
}

fn ensure_parent_dir(path: &str) -> Result<(), String> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create directory for {}: {}", path, e))?;
    }
    Ok(())
}

/// Download a single file from the remote server with progress reporting,
/// verifying its size and hash against the manifest before it goes in place.
pub async fn download_file(
    reporter: &Reporter,
    entry: &FileEntry,
    base_downloaded: u64,
    total_download_bytes: u64,
) -> Result<(), String> {
    let url = format!("{}{}", DOWNLOAD_BASE_URL, entry.path);

    let response = crate::backend::http()
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

    ensure_parent_dir(&entry.path)?;
    let partial = partial_path(&entry.path);
    let mut file = fs::File::create(&partial)
        .map_err(|e| format!("Cannot create file {}: {}", partial, e))?;

    let mut hasher = XxHash64::with_seed(0);
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download error for {}: {}", entry.path, e))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Write error for {}: {}", entry.path, e))?;
        hasher.write(&chunk);
        downloaded += chunk.len() as u64;

        reporter.send(SyncEvent::Downloading {
            bytes_done: base_downloaded + downloaded,
            total_bytes: total_download_bytes,
            current_file: entry.path.clone(),
        });
    }
    drop(file);

    let hash = format!("{:016x}", hasher.finish());
    if downloaded != entry.size || !hash.eq_ignore_ascii_case(&entry.xxhash) {
        let _ = fs::remove_file(&partial);
        return Err(format!(
            "{} failed verification after download ({} of {} bytes, hash {} instead of {})",
            entry.path, downloaded, entry.size, hash, entry.xxhash
        ));
    }

    replace_with(&partial, &entry.path)?;
    crate::lethe_log!("Downloaded: {} ({} bytes)", entry.path, downloaded);
    Ok(())
}

/// Download files sequentially (downloads are network-bound, not worth parallelizing).
pub async fn download_files(
    reporter: &Reporter,
    files: &[FileEntry],
    base_bytes: u64,
    total_bytes: u64,
) -> Result<u64, String> {
    let mut processed = base_bytes;

    for entry in files {
        // Try a matching local Steam copy first. Hashing it is blocking work.
        let steam_entry = entry.clone();
        let copied = tokio::task::spawn_blocking(move || try_copy_from_steam(&steam_entry))
            .await
            .map_err(|e| format!("Copying {} failed: {}", entry.path, e))??;
        if copied {
            processed += entry.size;
            reporter.send(SyncEvent::Copied {
                bytes_done: processed,
                total_bytes,
                current_file: entry.path.clone(),
            });
            continue;
        }

        download_file(reporter, entry, processed, total_bytes).await?;
        processed += entry.size;
    }

    Ok(processed)
}

/// Download the plugin DLLs that aren't part of the manifest (Lethe.dll,
/// ModularSkillScripts.dll, motions.dll) into BepInEx/plugins/.
pub async fn download_additional_dlls(reporter: &Reporter) -> Result<(), String> {
    let plugins_dir = Path::new("BepInEx").join("plugins");
    fs::create_dir_all(&plugins_dir)
        .map_err(|e| format!("Cannot create BepInEx/plugins: {}", e))?;

    for (url, filename) in ADDITIONAL_DLLS {
        reporter.status(format!("Downloading {}...", filename));

        // fetch_bytes rejects error pages (404, 502...) instead of saving them as the DLL.
        let bytes = crate::backend::fetch_bytes(url)
            .await
            .map_err(|e| format!("Failed to download {}: {}", filename, e))?;

        let file_path = plugins_dir.join(filename).to_string_lossy().into_owned();
        let partial = partial_path(&file_path);
        fs::write(&partial, &bytes).map_err(|e| format!("Cannot write {}: {}", filename, e))?;
        replace_with(&partial, &file_path)?;

        crate::lethe_log!(
            "Downloaded additional DLL: {} ({} bytes)",
            filename,
            bytes.len()
        );
    }

    Ok(())
}

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
    use crate::backend::manifest::FileManifest;

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
