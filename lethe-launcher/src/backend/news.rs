use serde::Deserialize;

/// A static file on the CDN, next to the game files, so launcher start-ups
/// don't hit the backend. If it is missing or malformed the launcher simply
/// hides the news card.
const NEWS_URL: &str = "https://files.lethelc.site/news.json";

/// Example `news.json`:
///
/// ```json
/// {
///   "banners": [
///     { "image": "https://.../event.png", "title": "Custom identities", "subtitle": "Mod by Water", "url": "https://..." }
///   ],
///   "items": [
///     { "title": "Server maintenance complete", "date": "11/04", "url": "https://..." }
///   ]
/// }
/// ```
#[derive(Debug, Clone, Default, Deserialize)]
pub struct NewsFeed {
    #[serde(default)]
    pub banners: Vec<Banner>,
    #[serde(default)]
    pub items: Vec<NewsItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Banner {
    pub image: String,
    /// Optional caption shown over the bottom of the banner.
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub subtitle: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewsItem {
    pub title: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub url: Option<String>,
}

/// A banner whose image bytes have already been downloaded.
#[derive(Debug, Clone)]
pub struct LoadedBanner {
    pub bytes: Vec<u8>,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub url: Option<String>,
}

pub async fn fetch_news() -> Result<NewsFeed, String> {
    let response = reqwest::get(NEWS_URL)
        .await
        .map_err(|e| format!("Failed to fetch news: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("News returned HTTP {}", response.status().as_u16()));
    }
    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read news: {}", e))?;
    parse_news(&body)
}

pub fn parse_news(body: &str) -> Result<NewsFeed, String> {
    serde_json::from_str(body).map_err(|e| format!("Failed to parse news: {}", e))
}

/// Download banner images, skipping any that fail.
pub async fn fetch_banners(banners: &[Banner]) -> Vec<LoadedBanner> {
    let mut loaded = Vec::new();
    for banner in banners.iter().take(5) {
        match crate::backend::fetch_bytes(&banner.image).await {
            Ok(bytes) => loaded.push(LoadedBanner {
                bytes,
                title: banner.title.clone(),
                subtitle: banner.subtitle.clone(),
                url: banner.url.clone(),
            }),
            Err(e) => crate::lethe_log!("Skipping banner {}: {}", banner.image, e),
        }
    }
    loaded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_feed() {
        let feed = parse_news(
            r#"{
                "banners": [{"image": "https://x/b.png", "title": "T", "url": "https://x"}],
                "items": [
                    {"title": "A", "date": "11/04", "url": "https://a"},
                    {"title": "B"}
                ]
            }"#,
        )
        .unwrap();
        assert_eq!(feed.banners.len(), 1);
        assert_eq!(feed.banners[0].title.as_deref(), Some("T"));
        assert!(feed.banners[0].subtitle.is_none());
        assert_eq!(feed.items[0].title, "A");
        assert_eq!(feed.items[1].date, "");
        assert!(feed.items[1].url.is_none());
    }

    #[test]
    fn missing_sections_default_to_empty() {
        let feed = parse_news("{}").unwrap();
        assert!(feed.banners.is_empty());
        assert!(feed.items.is_empty());
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_news("<html>").is_err());
    }
}
