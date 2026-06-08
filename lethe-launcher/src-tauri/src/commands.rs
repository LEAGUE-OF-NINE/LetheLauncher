use crate::downloader::{download_additional_dlls, download_files, StatusChangeEvent};
use crate::manifest::FileEntry;
use crate::verifier::{check_file, load_cache, save_cache, CheckResult};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Serialize, Clone)]
pub struct CheckProgressEvent {
    pub files_checked: u64,
    pub total_files: u64,
    pub bytes_processed: u64,
    pub total_bytes: u64,
    pub current_file: String,
    pub percent: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct SyncCompleteEvent;

#[derive(Debug, Serialize, Clone)]
pub struct ErrorEvent {
    pub message: String,
}

/// Main sync command: download manifest, check files, download missing, launch.
#[tauri::command]
pub async fn start_sync(app_handle: AppHandle) -> Result<(), String> {
    let sync_result = do_start_sync(app_handle.clone()).await;
    if let Err(ref e) = sync_result {
        let _ = app_handle.emit("error", ErrorEvent { message: e.clone() });
    }
    sync_result
}

async fn do_start_sync(app_handle: AppHandle) -> Result<(), String> {
    // 1. Download manifest
    let manifest = crate::downloader::download_manifest(&app_handle).await?;

    let total_files = manifest.files.len() as u64;
    let total_bytes = manifest.total_size;

    let _ = app_handle.emit(
        "status-change",
        StatusChangeEvent {
            message: format!("Checking {} files...", total_files),
        },
    );

    // 2. Load file cache
    let cache = load_cache();

    // 3. Parallel file checking with semaphore
    let files_checked = Arc::new(AtomicU64::new(0));
    let bytes_processed = Arc::new(AtomicU64::new(0));
    let semaphore = Arc::new(tokio::sync::Semaphore::new(4));

    let mut handles = Vec::new();
    let mut results: Vec<(FileEntry, CheckResult)> = Vec::new();

    // We need to check files one at a time for cache coherence,
    // but we can parallelize the I/O-heavy hash computation
    let file_entries = manifest.files;

    // Use a shared async-safe cache wrapper
    let cache = Arc::new(tokio::sync::Mutex::new(cache));

    for entry in file_entries {
        let app = app_handle.clone();
        let files_checked = files_checked.clone();
        let bytes_processed = bytes_processed.clone();
        let semaphore = semaphore.clone();
        let cache = cache.clone();
        let total_files = total_files;
        let total_bytes = total_bytes;

        let handle = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();

            let result = {
                let mut cache_guard = cache.lock().await;
                check_file(&entry, &mut cache_guard)
            };

            match result {
                CheckResult::Valid => {
                    bytes_processed.fetch_add(entry.size, Ordering::SeqCst);
                }
                CheckResult::Missing | CheckResult::Mismatched => {
                    // Don't count bytes yet - will be counted during download
                }
            }

            let checked = files_checked.fetch_add(1, Ordering::SeqCst) + 1;
            let processed = bytes_processed.load(Ordering::SeqCst);
            let percent = if total_bytes > 0 {
                (processed as f64 / total_bytes as f64) * 100.0
            } else {
                0.0
            };

            let _ = app.emit(
                "check-progress",
                CheckProgressEvent {
                    files_checked: checked,
                    total_files,
                    bytes_processed: processed,
                    total_bytes,
                    current_file: entry.path.clone(),
                    percent,
                },
            );

            (entry, result)
        });

        handles.push(handle);
    }

    // Collect results
    for handle in handles {
        match handle.await {
            Ok((entry, result)) => {
                results.push((entry, result));
            }
            Err(e) => {
                crate::lethe_log!("Task panicked: {}", e);
            }
        }
    }

    // Separate mismatched/missing from valid
    let mut files_to_download: Vec<FileEntry> = Vec::new();
    let mut processed_bytes: u64 = 0;

    for (entry, result) in &results {
        match result {
            CheckResult::Valid => {
                processed_bytes += entry.size;
            }
            CheckResult::Mismatched => {
                crate::lethe_log!("File hash mismatch: {}", entry.path);
                files_to_download.push(entry.clone());
            }
            CheckResult::Missing => {
                crate::lethe_log!("File missing: {}", entry.path);
                files_to_download.push(entry.clone());
            }
        }
    }

    // Save cache after checking
    let cache = cache.lock().await;
    save_cache(&cache);
    drop(cache);

    // 4. Download missing/corrupted files
    if !files_to_download.is_empty() {
        let _ = app_handle.emit(
            "status-change",
            StatusChangeEvent {
                message: format!("Downloading {} files...", files_to_download.len()),
            },
        );

        let download_count = files_to_download.len() as u64;
        crate::lethe_log!("Need to download {} files", download_count);

        download_files(
            &app_handle,
            &files_to_download,
            processed_bytes,
            total_bytes,
        )
        .await?;

        let _ = app_handle.emit(
            "status-change",
            StatusChangeEvent {
                message: "Download complete!".to_string(),
            },
        );
    } else {
        let _ = app_handle.emit(
            "status-change",
            StatusChangeEvent {
                message: "All files are up to date!".to_string(),
            },
        );
    }

    // 5. Download additional DLLs
    download_additional_dlls(&app_handle).await?;

    // 6. Signal completion
    let _ = app_handle.emit("sync-complete", SyncCompleteEvent);

    crate::lethe_log!("Sync complete");
    Ok(())
}

/// Hide the window, launch the game, then close.
/// Optionally pass an auth token as LETHE_TOKEN env var for the BepInEx mod.
#[tauri::command]
pub async fn launch(app_handle: AppHandle, token: Option<String>) -> Result<(), String> {
    // Hide window
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.hide();
    }

    // Small delay to let UI finish updating
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    crate::lethe_log!("Update complete. Starting game...");

    // Launch the game (this blocks until game exits)
    crate::launcher::launch_game(token.as_deref())?;

    // Close the launcher after game exits
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.close();
    }

    Ok(())
}

/// Get all launcher settings.
#[tauri::command]
pub fn get_settings() -> std::collections::HashMap<String, String> {
    crate::config::read_config()
}

/// Set a launcher setting.
#[tauri::command]
pub fn set_setting(key: String, value: String) {
    crate::config::set_config_value(&key, &value);
}
#[tauri::command]
pub fn get_auto_update_status() -> bool {
    crate::config::is_auto_update_disabled()
}

/// Start Discord OAuth login flow. Returns auth result with username, token, and avatar URL.
/// The frontend decodes the JWT to extract the display name and avatar.
#[tauri::command]
pub async fn start_oauth() -> Result<crate::auth::AuthResult, String> {
    crate::auth::start_oauth_flow().await
}

/// Check if user is logged in (has saved token).
#[tauri::command]
pub fn get_saved_auth() -> Option<crate::auth::AuthResult> {
    crate::auth::load_saved_auth()
}

/// Log out (clear saved token).
#[tauri::command]
pub fn logout() {
    crate::auth::clear_saved_auth();
}

/// Check for launcher updates. Returns update info if a newer version is available.
#[tauri::command]
pub async fn check_for_updates() -> Result<Option<crate::updater::UpdateInfo>, String> {
    crate::updater::check_for_updates(env!("CARGO_PKG_VERSION")).await
}

/// Download and apply an update. The launcher will exit after this.
#[tauri::command]
pub async fn download_update(update: crate::updater::UpdateInfo) -> Result<(), String> {
    crate::updater::download_update(&update).await
}
