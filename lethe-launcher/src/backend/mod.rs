//! Launcher logic with no UI: file sync, auth, self-update, mods, news.
//! Async work runs on a shared tokio runtime because reqwest needs one;
//! the UI awaits the returned handles from GPUI's executor.

pub mod auth;
pub mod config;
pub mod downloader;
pub mod launcher;
pub mod logger;
pub mod manifest;
pub mod mods;
pub mod news;
pub mod steam;
pub mod sync;
pub mod updater;
pub mod verifier;

use std::future::Future;
use std::sync::OnceLock;

fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .thread_name("lethe-worker")
            .build()
            .expect("failed to start tokio runtime")
    })
}

/// Run a future on the backend runtime. Awaiting the result from any executor
/// yields the future's output; a panic inside it becomes an `Err`.
pub async fn run<F, T>(future: F) -> Result<T, String>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let handle = runtime().spawn(future);
    // Dropping the awaiting future (e.g. the user cancelled) stops the task too.
    let _abort = AbortOnDrop(handle.abort_handle());
    handle
        .await
        .map_err(|e| format!("Background task failed: {}", e))
}

struct AbortOnDrop(tokio::task::AbortHandle);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// One HTTP client for the whole launcher, so downloads reuse connections
/// instead of paying a new TLS handshake per file.
pub fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

/// Fetch a URL into memory. Fails on any non-success HTTP status.
pub async fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    let response = http()
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status().as_u16()));
    }
    response
        .bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| format!("Read failed: {}", e))
}

/// Whether `url` is a plain http(s) link. Links also come from the remote
/// news feed, and the Windows shell would just as happily run a file path,
/// a network share or another URL scheme.
pub fn is_web_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    let rest = match lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
    {
        Some(rest) => rest,
        None => return false,
    };
    !rest.is_empty() && !url.chars().any(|c| c.is_whitespace() || c.is_control() || c == '"')
}

/// Open a web link in the user's default browser.
pub fn open_url(url: &str) -> Result<(), String> {
    if !is_web_url(url) {
        crate::lethe_log!("Refusing to open non-web link: {}", url);
        return Err(format!("Not a web link: {}", url));
    }
    crate::lethe_log!("Opening URL: {}", url);
    #[cfg(target_os = "windows")]
    {
        // Arguments go straight to rundll32, never through cmd, so `&` in a
        // query string stays part of the URL.
        std::process::Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn()
            .map_err(|e| format!("Failed to open browser: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Failed to open browser: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Failed to open browser: {}", e))?;
    }
    Ok(())
}

pub fn format_bytes(bytes: u64) -> String {
    downloader::format_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::is_web_url;

    #[test]
    fn only_http_links_are_opened() {
        assert!(is_web_url("https://lethelc.site"));
        assert!(is_web_url("HTTP://example.com/a?b=1&c=2"));
        assert!(!is_web_url("https://"));
        assert!(!is_web_url("file:///C:/Windows/System32/calc.exe"));
        assert!(!is_web_url(r"\\host\share\x.exe"));
        assert!(!is_web_url(r"C:\payload.exe"));
        assert!(!is_web_url("javascript:alert(1)"));
        assert!(!is_web_url("https://a.com/ x.exe"));
        assert!(!is_web_url("https://a.com/\"x"));
    }
}
