use crate::backend::downloader::{download_additional_dlls, download_files};
use crate::backend::manifest::FileEntry;
use crate::backend::verifier::{check_file_cached, load_cache, save_cache, CheckResult};
use futures_util::StreamExt;

/// Progress updates sent from the sync task to the UI.
#[derive(Debug, Clone)]
pub enum SyncEvent {
    Status(String),
    Checking {
        files_checked: u64,
        total_files: u64,
        bytes_done: u64,
        total_bytes: u64,
        current_file: String,
    },
    Downloading {
        bytes_done: u64,
        total_bytes: u64,
        current_file: String,
    },
    /// A file was copied from the local Steam install instead of downloaded.
    Copied {
        bytes_done: u64,
        total_bytes: u64,
        current_file: String,
    },
}

/// Sends sync progress to the UI. Sending never blocks and never fails loudly:
/// if the UI went away there is nobody left to tell.
#[derive(Clone)]
pub struct Reporter(async_channel::Sender<SyncEvent>);

impl Reporter {
    pub fn new() -> (Self, async_channel::Receiver<SyncEvent>) {
        let (tx, rx) = async_channel::unbounded();
        (Self(tx), rx)
    }

    pub fn send(&self, event: SyncEvent) {
        let _ = self.0.try_send(event);
    }

    pub fn status(&self, message: impl Into<String>) {
        self.send(SyncEvent::Status(message.into()));
    }
}

/// How a successful sync ended, for the "ready" line in the UI.
#[derive(Debug, Clone)]
pub struct SyncSummary {
    pub total_files: u64,
    pub downloaded_files: u64,
}

/// Download the manifest, check every file, download missing/corrupted ones,
/// then fetch the extra plugin DLLs.
pub async fn run_sync(reporter: Reporter) -> Result<SyncSummary, String> {
    // 1. Download manifest
    let manifest = crate::backend::downloader::download_manifest(&reporter).await?;

    let total_files = manifest.files.len() as u64;
    let total_bytes = manifest.total_size;

    reporter.status(format!("Checking {} files...", total_files));

    // 2. Look up each file's cached hash up front, so the checks below don't
    //    share (or lock) the cache.
    let mut cache = load_cache();
    let jobs: Vec<(FileEntry, _)> = manifest
        .files
        .into_iter()
        .map(|entry| {
            let cached = cache.entries.get(&entry.path).cloned();
            (entry, cached)
        })
        .collect();

    // 3. Hash four files at a time on the blocking pool. The checks live in
    //    this future, so cancelling the sync stops them; only hashes already
    //    in progress run to completion.
    let mut checks = futures_util::stream::iter(jobs.into_iter().map(|(entry, cached)| {
        tokio::task::spawn_blocking(move || {
            let (result, update) = check_file_cached(&entry, cached.as_ref());
            (entry, result, update)
        })
    }))
    .buffer_unordered(4);

    let mut files_checked: u64 = 0;
    let mut processed_bytes: u64 = 0;
    let mut files_to_download: Vec<FileEntry> = Vec::new();
    while let Some(joined) = checks.next().await {
        // A check that panicked can't vouch for its file: fail the sync
        // rather than report it as verified.
        let (entry, result, update) = joined.map_err(|e| format!("File check failed: {}", e))?;
        if let Some(info) = update {
            cache.entries.insert(entry.path.clone(), info);
        }
        files_checked += 1;
        if result == CheckResult::Valid {
            processed_bytes += entry.size;
        }
        reporter.send(SyncEvent::Checking {
            files_checked,
            total_files,
            bytes_done: processed_bytes,
            total_bytes,
            current_file: entry.path.clone(),
        });
        match result {
            CheckResult::Valid => {}
            CheckResult::Mismatched => {
                crate::lethe_log!("File hash mismatch: {}", entry.path);
                files_to_download.push(entry);
            }
            CheckResult::Missing => {
                crate::lethe_log!("File missing: {}", entry.path);
                files_to_download.push(entry);
            }
        }
    }
    drop(checks);

    save_cache(&cache);

    // 4. Download missing/corrupted files
    let downloaded_files = files_to_download.len() as u64;
    if !files_to_download.is_empty() {
        reporter.status(format!("Downloading {} files...", downloaded_files));
        crate::lethe_log!("Need to download {} files", downloaded_files);
        download_files(&reporter, &files_to_download, processed_bytes, total_bytes).await?;
    }

    // 5. Download additional DLLs
    download_additional_dlls(&reporter).await?;

    crate::lethe_log!("Sync complete");
    Ok(SyncSummary {
        total_files,
        downloaded_files,
    })
}
