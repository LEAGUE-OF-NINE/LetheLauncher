//! Bottom-left news card: rotating banner plus the latest headlines.
//! Hidden entirely until `news.json` loads, so a missing feed costs nothing.
//! Styled like the site's cards: notched corners, a "SERVER STATUS"-style
//! header with a red rule, hairline rows, and captions with a red accent bar.

use crate::backend::{self, news::NewsFeed};
use crate::ui::theme::{self, black_a, white_a};
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::sync::Arc;
use std::time::Duration;

const BANNER_INTERVAL: Duration = Duration::from_secs(7);
const NOTCH: f32 = 14.;

struct Banner {
    image: Arc<Image>,
    title: Option<String>,
    subtitle: Option<String>,
    url: Option<String>,
}

pub struct NewsCard {
    feed: Option<NewsFeed>,
    banners: Vec<Banner>,
    banner_index: usize,
    _tasks: Vec<Task<()>>,
}

impl NewsCard {
    /// Debug builds only: fake feed for `LETHE_DEMO`.
    #[cfg(debug_assertions)]
    pub fn load_demo(&mut self, cx: &mut Context<Self>) {
        use crate::backend::news::NewsItem;
        // Drop the real fetch so it can't replace the demo feed.
        drop(self._tasks.remove(0));
        let item = |title: &str, date: &str| NewsItem {
            title: title.into(),
            date: date.into(),
            url: Some("https://lethelc.site".into()),
        };
        self.feed = Some(NewsFeed {
            banners: Vec::new(),
            items: vec![
                item("Server maintenance complete", "11/04"),
                item("Patch 1.4.2: battle fixes and new identities", "11/03"),
                item("Lethe launcher rebuilt from scratch", "11/02"),
                item("Community tournament sign-ups open", "10/28"),
                item("New mod: Better UI", "10/29"),
            ],
        });
        self.banners = ["Custom identities and skills", "Refraction Railway", "Mirror Dungeon"]
            .iter()
            .map(|title| Banner {
                image: super::background_image(),
                title: Some(title.to_string()),
                subtitle: Some("Placeholder banner from the demo feed".into()),
                url: None,
            })
            .collect();
        cx.notify();
    }

    pub fn new(cx: &mut Context<Self>) -> Self {
        let load = cx.spawn(async move |this, cx| {
            let feed = match backend::run(backend::news::fetch_news()).await.and_then(|r| r) {
                Ok(feed) => feed,
                Err(e) => {
                    crate::lethe_log!("News unavailable: {}", e);
                    return;
                }
            };
            let banner_specs = feed.banners.clone();
            let _ = this.update(cx, |this, cx| {
                this.feed = Some(feed);
                cx.notify();
            });

            let loaded = backend::run(async move { backend::news::fetch_banners(&banner_specs).await })
                .await
                .unwrap_or_default();
            let _ = this.update(cx, |this, cx| {
                this.banners = loaded
                    .into_iter()
                    .filter_map(|b| {
                        super::image_from_bytes(b.bytes).map(|image| Banner {
                            image,
                            title: b.title,
                            subtitle: b.subtitle,
                            url: b.url,
                        })
                    })
                    .collect();
                cx.notify();
            });
        });

        let rotate = cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(BANNER_INTERVAL).await;
            let alive = this.update(cx, |this, cx| {
                if this.banners.len() > 1 {
                    this.banner_index = (this.banner_index + 1) % this.banners.len();
                    cx.notify();
                }
            });
            if alive.is_err() {
                break;
            }
        });

        Self {
            feed: None,
            banners: Vec::new(),
            banner_index: 0,
            _tasks: vec![load, rotate],
        }
    }

    fn render_banner(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let banner = self.banners.get(self.banner_index)?;
        let url = banner.url.clone();
        let has_caption = banner.title.is_some() || banner.subtitle.is_some();

        Some(
            div()
                .id("banner")
                .relative()
                .w_full()
                .h(px(150.))
                .overflow_hidden()
                .when(url.is_some(), |d| d.cursor_pointer())
                .on_click(move |_, _, _| {
                    if let Some(url) = &url {
                        let _ = backend::open_url(url);
                    }
                })
                .child(img(banner.image.clone()).size_full().object_fit(ObjectFit::Cover))
                .when(has_caption, |d| {
                    d.child(
                        v_flex()
                            .absolute()
                            .left_0()
                            .right_0()
                            .bottom_0()
                            .px(px(12.))
                            .py(px(7.))
                            .bg(black_a(0.78))
                            .border_l_2()
                            .border_color(theme::primary())
                            .when_some(banner.title.clone(), |c, title| {
                                c.child(
                                    div()
                                        .font_family(theme::HEADING_FONT)
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_size(px(17.))
                                        .line_height(px(19.))
                                        .text_color(theme::foreground())
                                        .truncate()
                                        .child(title.to_uppercase()),
                                )
                            })
                            .when_some(banner.subtitle.clone(), |c, sub| {
                                c.child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(theme::muted_foreground())
                                        .truncate()
                                        .child(sub),
                                )
                            }),
                    )
                })
                .when(self.banners.len() > 1, |d| {
                    d.child(
                        h_flex()
                            .absolute()
                            .right(px(10.))
                            .top(px(10.))
                            .gap(px(4.))
                            .children((0..self.banners.len()).map(|i| {
                                let active = i == self.banner_index;
                                div()
                                    .id(("dot", i))
                                    .w(px(16.))
                                    .h(px(3.))
                                    .cursor_pointer()
                                    .bg(if active { theme::primary() } else { white_a(0.45) })
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        // The dots sit on the banner; don't also open its link.
                                        cx.stop_propagation();
                                        this.banner_index = i;
                                        cx.notify();
                                    }))
                            })),
                    )
                }),
        )
    }
}

impl Render for NewsCard {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(feed) = &self.feed else {
            return div().into_any_element();
        };
        if feed.items.is_empty() && self.banners.is_empty() {
            return div().into_any_element();
        }

        let items: Vec<_> = feed.items.iter().take(4).cloned().collect();

        theme::notched(NOTCH, theme::card().opacity(0.92), theme::border())
            .w_full()
            .child(
                // Inset so the banner doesn't cover the notched outline.
                v_flex()
                    .relative()
                    .p(px(1.))
                    .children(self.render_banner(cx))
                    .child(
                        h_flex()
                            .px(px(12.))
                            .pt(px(10.))
                            .border_b_1()
                            .border_color(theme::primary().opacity(0.55))
                            .child(
                                div()
                                    .pb(px(7.))
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme::foreground())
                                    .border_b_2()
                                    .border_color(theme::primary())
                                    .child(theme::label_text("News")),
                            ),
                    )
                    .child(
                        v_flex()
                            .pb(px(NOTCH / 2.))
                            .when(items.is_empty(), |list| {
                                list.child(
                                    div()
                                        .px(px(12.))
                                        .py(px(10.))
                                        .text_size(px(12.))
                                        .text_color(theme::muted_foreground())
                                        .child("No news yet"),
                                )
                            })
                            .children(items.into_iter().enumerate().map(|(i, item)| {
                                let url = item.url.clone();
                                h_flex()
                                    .id(("news-item", i))
                                    .px(px(12.))
                                    .py(px(7.))
                                    .gap(px(12.))
                                    .justify_between()
                                    .text_size(px(13.))
                                    .when(i > 0, |row| row.border_t_1().border_color(white_a(0.06)))
                                    .when(url.is_some(), |row| {
                                        row.cursor_pointer().hover(|s| s.bg(white_a(0.04)))
                                    })
                                    .on_click(move |_, _, _| {
                                        if let Some(url) = &url {
                                            let _ = backend::open_url(url);
                                        }
                                    })
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .truncate()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme::foreground())
                                            .child(item.title),
                                    )
                                    .child(
                                        div()
                                            .flex_none()
                                            .text_size(px(11.))
                                            .text_color(theme::muted_foreground())
                                            .child(item.date),
                                    )
                            })),
                    ),
            )
            .into_any_element()
    }
}
