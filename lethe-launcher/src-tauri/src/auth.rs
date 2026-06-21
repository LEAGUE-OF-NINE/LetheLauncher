use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const AUTH_SERVER: &str = "https://api.lethelc.site";
const AUTH_FILE: &str = "lethe-auth.json";
const POLL_INTERVAL_MS: u64 = 1000;
const LOGIN_TIMEOUT_SECS: u64 = 120;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthResult {
    pub username: String,
    pub token: String,
    pub avatar_url: String,
}

#[derive(Debug, Deserialize)]
struct PollResponse {
    token: Option<String>,
}

/// Start the OAuth flow using session-based polling (same as Lethe mod's LoginCoroutine).
/// Generates a session_id, opens the browser, polls for token, returns auth result.
pub async fn start_oauth_flow() -> Result<AuthResult, String> {
    // 1. Generate a random session ID (same algorithm as LetheHooks.GenerateSessionID)
    let session_id = generate_session_id();
    let login_url = format!("{}/auth/login?session_id={}&launcher=true", AUTH_SERVER, session_id);

    // 2. Open browser
    crate::lethe_log!("Opening browser: {}", login_url);
    open_browser(&login_url)?;

    // 3. Poll for token
    let poll_url = format!("{}/auth/token/poll", AUTH_SERVER);
    crate::lethe_log!("Polling {} for token (timeout: {}s)...", poll_url, LOGIN_TIMEOUT_SECS);

    let client = reqwest::Client::new();
    let start = Instant::now();
    let mut attempts = 0u32;

    loop {
        attempts += 1;
        let elapsed = start.elapsed();

        if elapsed > Duration::from_secs(LOGIN_TIMEOUT_SECS) {
            return Err(format!(
                "Login timed out after {}s ({} attempts). Did you complete Discord authorization?",
                LOGIN_TIMEOUT_SECS, attempts
            ));
        }

        let body = serde_json::json!({ "session_id": &session_id });
        match client.post(&poll_url).json(&body).send().await {
            Ok(resp) => {
                match resp.status().as_u16() {
                    200 => {
                        let body = resp.text().await
                            .map_err(|e| format!("Failed to read poll response: {}", e))?;
                        crate::lethe_log!("Poll response body: {}", &body[..body.len().min(500)]);
                        let data: PollResponse = serde_json::from_str(&body)
                            .map_err(|e| format!("Failed to parse poll response: {}", e))?;
                        if let Some(t) = data.token {
                            crate::lethe_log!("Token received after {} poll(s)", attempts);
                            // Extract user info from JWT claims (poll only returns token)
                            let claims = jwt_claims(&t);
                            let user_id = claims.sub.unwrap_or_default();
                            let username = claims.name.unwrap_or_else(|| user_id.clone());
                            let avatar_url = if !user_id.is_empty() {
                                if let Some(ref hash) = claims.avatar {
                                    format!("https://cdn.discordapp.com/avatars/{}/{}.webp", user_id, hash)
                                } else {
                                    let uid: u64 = user_id.parse().unwrap_or(0);
                                    format!("https://cdn.discordapp.com/embed/avatars/{}.png", (uid >> 22) % 6)
                                }
                            } else {
                                String::new()
                            };
                            return Ok(AuthResult { username, token: t, avatar_url });
                        }
                        // 200 but no token field - keep polling
                        crate::lethe_log!("Poll returned 200 but no token in response");
                    }
                    404 => {
                        // Expected: user hasn't logged in yet, silently retry
                    }
                    code => {
                        if attempts % 10 == 1 {
                            crate::lethe_log!("Poll attempt {}: server returned HTTP {}", attempts, code);
                        }
                    }
                }
            }
            Err(e) => {
                if attempts % 10 == 1 {
                    crate::lethe_log!("Poll attempt {} failed: {}", attempts, e);
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
    }
}

#[derive(Debug, Default)]
struct JwtClaims {
    sub: Option<String>,
    name: Option<String>,
    avatar: Option<String>,
}

/// Extract user info from JWT payload without verification (display-only).
fn jwt_claims(token: &str) -> JwtClaims {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return JwtClaims::default();
    }
    let decoded = base64_decode_url(parts[1]).unwrap_or_default();
    let json: serde_json::Value = serde_json::from_str(&decoded).unwrap_or_default();
    JwtClaims {
        sub: json.get("sub").and_then(|v| v.as_str()).map(|s| s.to_string()),
        name: json.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()),
        avatar: json.get("avatar").and_then(|v| v.as_str()).map(|s| s.to_string()),
    }
}

fn generate_session_id() -> String {
    let mut buffer = [0u8; 32];
    rand::thread_rng().fill(&mut buffer);
    // URL-safe base64 without padding (same as C# Convert.ToBase64String + TrimEnd('=') + Replace)
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buffer)
}

pub fn load_saved_auth() -> Option<AuthResult> {
    load_saved_auth_from(AUTH_FILE)
}

pub fn load_saved_auth_from(path: &str) -> Option<AuthResult> {
    if let Ok(contents) = std::fs::read_to_string(path) {
        if let Ok(auth) = serde_json::from_str::<AuthResult>(&contents) {
            if let Some(true) = is_token_expired(&auth.token) {
                let _ = std::fs::remove_file(path);
                return None;
            }
            return Some(auth);
        }
    }
    None
}

pub fn save_auth(auth: &AuthResult) {
    save_auth_to(auth, AUTH_FILE);
}

pub fn save_auth_to(auth: &AuthResult, path: &str) {
    if let Ok(json) = serde_json::to_string_pretty(auth) {
        let _ = std::fs::write(path, json);
    }
}

pub fn clear_saved_auth() {
    let _ = std::fs::remove_file(AUTH_FILE);
}

/// Extract username from JWT payload. Tries common Discord OAuth claims.
#[allow(dead_code)]
pub fn extract_jwt_username(token: &str) -> Option<String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 { return None; }
    let decoded = base64_decode_url(parts[1])?;
    let json: serde_json::Value = serde_json::from_str(&decoded).ok()?;
    // Try common Discord/OAuth claim names
    json.get("username")
        .or_else(|| json.get("name"))
        .or_else(|| json.get("global_name"))
        .or_else(|| json.get("sub"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn is_token_expired(token: &str) -> Option<bool> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 { return None; }
    let payload = parts[1];
    let decoded = base64_decode_url(payload)?;
    let json: serde_json::Value = serde_json::from_str(&decoded).ok()?;
    let exp = json.get("exp")?.as_i64()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).ok()?
        .as_secs() as i64;
    Some(now >= exp)
}

fn base64_decode_url(input: &str) -> Option<String> {
    let mut b64 = input.replace('-', "+").replace('_', "/");
    while b64.len() % 4 != 0 { b64.push('='); }
    let mut decoded = Vec::new();
    let decode_table = |c: u8| -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };
    let bytes: Vec<u8> = b64.bytes().filter(|&b| b != b'=').collect();
    for chunk in bytes.chunks(4) {
        if chunk.len() < 2 { break; }
        let b0 = decode_table(chunk[0])?;
        let b1 = decode_table(chunk[1])?;
        let b2 = if chunk.len() > 2 { decode_table(chunk[2]) } else { None };
        let b3 = if chunk.len() > 3 { decode_table(chunk[3]) } else { None };
        decoded.push((b0 << 2) | (b1 >> 4));
        if let Some(b2) = b2 { decoded.push(((b1 & 0x0F) << 4) | (b2 >> 2)); }
        if let (Some(b2), Some(b3)) = (b2, b3) { decoded.push(((b2 & 0x03) << 6) | b3); }
    }
    String::from_utf8(decoded).ok()
}

fn open_browser(url: &str) -> Result<(), String> {
    crate::lethe_log!("Opening URL: {}", url);
    #[cfg(target_os = "windows")]
    {
        let result = std::process::Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn();
        if result.is_err() {
            std::process::Command::new("cmd")
                .args(["/c", "start", "", url])
                .spawn()
                .map_err(|e| format!("Failed to open browser: {}", e))?;
        } else {
            crate::lethe_log!("Browser launched");
        }
    }
    #[cfg(target_os = "macos")]
    { std::process::Command::new("open").arg(url).spawn()
        .map_err(|e| format!("Failed to open browser: {}", e))?; }
    #[cfg(target_os = "linux")]
    { std::process::Command::new("xdg-open").arg(url).spawn()
        .map_err(|e| format!("Failed to open browser: {}", e))?; }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_decode_hello() {
        assert_eq!(base64_decode_url("SGVsbG8").unwrap(), "Hello");
    }

    #[test]
    fn test_base64_decode_json() {
        let payload = "eyJleHAiOjk5OTk5OTk5OTl9";
        let decoded = base64_decode_url(payload).unwrap();
        assert!(decoded.contains("exp"));
        assert!(decoded.contains("9999999999"));
    }

    #[test]
    fn test_base64_decode_url_safe() {
        let decoded = base64_decode_url("SGVsbG8tV29ybGQ_").unwrap();
        assert_eq!(decoded, "Hello-World?");
    }

    #[test]
    fn test_token_expired_future() {
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjI5OTk5OTk5OTksInN1YiI6InRlc3QifQ.sig";
        assert_eq!(is_token_expired(token), Some(false));
    }

    #[test]
    fn test_token_expired_past() {
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjk0NjY4NDgwMH0.sig";
        assert_eq!(is_token_expired(token), Some(true));
    }

    #[test]
    fn test_token_expired_invalid() {
        assert_eq!(is_token_expired("not.jwt"), None);
        assert_eq!(is_token_expired("garbage"), None);
        assert_eq!(is_token_expired(""), None);
    }

    #[test]
    fn test_save_load_clear_auth() {
        let test_file = "test-auth-save-load.json";
        let auth = AuthResult {
            username: "TestUser".into(),
            token: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjI5OTk5OTk5OTksInN1YiI6InRlc3QifQ.sig".into(),
            avatar_url: "https://cdn.discordapp.com/avatars/123/456.webp".into(),
        };
        save_auth_to(&auth, test_file);
        let loaded = load_saved_auth_from(test_file).unwrap();
        assert_eq!(loaded.username, "TestUser");
        std::fs::remove_file(test_file).unwrap();
        assert!(load_saved_auth_from(test_file).is_none());
    }

    #[test]
    fn test_load_expired_auth_returns_none() {
        let test_file = "test-auth-expired.json";
        let auth = AuthResult {
            username: "Old".into(),
            token: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjk0NjY4NDgwMH0.sig".into(),
            avatar_url: String::new(),
        };
        save_auth_to(&auth, test_file);
        assert!(load_saved_auth_from(test_file).is_none());
    }

    #[test]
    fn test_generate_session_id() {
        let id1 = generate_session_id();
        let id2 = generate_session_id();
        assert!(!id1.is_empty());
        assert_ne!(id1, id2);
        assert!(!id1.contains('='));
        assert!(!id1.contains('+'));
        assert!(!id1.contains('/'));
    }

    #[test]
    fn test_extract_jwt_username() {
        // JWT with {"username":"TestPlayer","sub":"12345"}
        let token = "eyJhbGciOiJIUzI1NiJ9.eyJ1c2VybmFtZSI6IlRlc3RQbGF5ZXIiLCJzdWIiOiIxMjM0NSJ9.sig";
        assert_eq!(extract_jwt_username(token), Some("TestPlayer".into()));

        // JWT with only sub claim
        let token2 = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiI2Nzg5MCJ9.sig";
        assert_eq!(extract_jwt_username(token2), Some("67890".into()));

        // JWT with no recognizable claims
        let token3 = "eyJhbGciOiJIUzI1NiJ9.eyJleHAiOjk5OTk5OTk5OTl9.sig";
        assert_eq!(extract_jwt_username(token3), None);
    }
}
