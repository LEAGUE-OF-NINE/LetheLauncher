//! Visual language shared with lethelc.site: near-black cool surfaces, a
//! crimson primary, gold for links and good news, square corners, notched
//! cards, Barlow Condensed headings over Inter body text.
//!
//! Every color and font the launcher uses comes from here, so when the site
//! is restyled the launcher can follow by editing this one file.

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::*;
use std::borrow::Cow;

/// Barlow Condensed. DirectWrite files it under the family "Barlow" with a
/// condensed stretch (the font has no explicit WWS family name), so that is
/// the name GPUI has to ask for on Windows.
pub const HEADING_FONT: &str = "Barlow";
pub const BODY_FONT: &str = "Inter";

// Site tokens (`.dark` in the site's CSS, converted from OKLCH).
pub fn background() -> Hsla {
    rgb(0x09090B).into()
}
pub fn card() -> Hsla {
    rgb(0x111115).into()
}
pub fn secondary() -> Hsla {
    rgb(0x1A1A1F).into()
}
pub fn sidebar() -> Hsla {
    rgb(0x040406).into()
}
pub fn foreground() -> Hsla {
    rgb(0xE9E8E4).into()
}
pub fn muted_foreground() -> Hsla {
    rgb(0xA19E98).into()
}
pub fn primary() -> Hsla {
    rgb(0xD01D21).into()
}
pub fn primary_hover() -> Hsla {
    rgb(0xE23532).into()
}
pub fn primary_active() -> Hsla {
    rgb(0xBB0916).into()
}
pub fn primary_foreground() -> Hsla {
    rgb(0xFEF2F2).into()
}
pub fn destructive() -> Hsla {
    rgb(0xFF6467).into()
}
pub fn gold() -> Hsla {
    rgb(0xDBB155).into()
}
pub fn border() -> Hsla {
    white_a(0.12)
}

/// White at the given opacity, for text and hairlines over art.
pub fn white_a(alpha: f32) -> Hsla {
    hsla(0., 0., 1., alpha)
}
/// Black at the given opacity, for scrims over art.
pub fn black_a(alpha: f32) -> Hsla {
    hsla(0., 0., 0., alpha)
}

/// Register the bundled fonts. Both are SIL Open Font License (see assets/fonts).
pub fn load_fonts(cx: &mut App) {
    let fonts: Vec<Cow<'static, [u8]>> = vec![
        Cow::Borrowed(include_bytes!("../../assets/fonts/BarlowCondensed-Medium.ttf")),
        Cow::Borrowed(include_bytes!("../../assets/fonts/BarlowCondensed-SemiBold.ttf")),
        Cow::Borrowed(include_bytes!("../../assets/fonts/BarlowCondensed-Bold.ttf")),
        Cow::Borrowed(include_bytes!("../../assets/fonts/Inter-Regular.ttf")),
        Cow::Borrowed(include_bytes!("../../assets/fonts/Inter-Medium.ttf")),
        Cow::Borrowed(include_bytes!("../../assets/fonts/Inter-SemiBold.ttf")),
    ];
    if let Err(e) = cx.text_system().add_fonts(fonts) {
        crate::lethe_log!("Failed to load bundled fonts: {}", e);
    }
}

pub fn apply(cx: &mut App) {
    Theme::change(ThemeMode::Dark, None, cx);
    Theme::update(cx, |t| {
        t.font_family = BODY_FONT.into();
        t.radius = px(0.);
        t.radius_lg = px(0.);
        t.shadow = false;

        t.background = background();
        t.foreground = foreground();
        t.muted = rgb(0x18181C).into();
        t.muted_foreground = muted_foreground();
        t.border = border();
        t.input = white_a(0.15);
        t.popover = card();
        t.popover_foreground = foreground();
        t.accent = rgb(0x1F1F24).into();
        t.accent_foreground = foreground();

        t.primary = primary();
        t.primary_hover = primary_hover();
        t.primary_active = primary_active();
        t.primary_foreground = primary_foreground();
        // The dark theme ships its own primary button colors.
        t.button_primary = primary();
        t.button_primary_hover = primary_hover();
        t.button_primary_active = primary_active();
        t.button_primary_foreground = primary_foreground();

        t.secondary = secondary();
        t.secondary_hover = rgb(0x24242A).into();
        t.secondary_active = rgb(0x2A2A31).into();
        t.secondary_foreground = foreground();

        t.danger = destructive();
        t.ring = gold();
        t.progress_bar = primary();
        t.switch = white_a(0.16);
        t.title_bar = sidebar();
        t.title_bar_border = border();
    });
}

/// Fake letter-spacing (GPUI has none) by putting `gap` between letters and
/// `word_gap` between words.
pub fn tracked(text: &str, gap: &str, word_gap: &str) -> String {
    let mut out = String::new();
    for (w, word) in text.split(' ').filter(|w| !w.is_empty()).enumerate() {
        if w > 0 {
            out.push_str(word_gap);
        }
        for (i, ch) in word.chars().enumerate() {
            if i > 0 {
                out.push_str(gap);
            }
            out.push(ch);
        }
    }
    out
}

/// Small uppercase Inter label, like the site's "SERVER STATUS" headers.
/// Hair spaces give it roughly the site's 0.1em tracking.
pub fn label_text(text: &str) -> String {
    tracked(&text.to_uppercase(), "\u{200A}", " \u{200A}")
}

/// Barlow Condensed heading with wide tracking, like "W E L C O M E  T O".
/// Uses regular spaces, which are 0.2em in Barlow Condensed.
pub fn wide_heading_text(text: &str) -> String {
    tracked(&text.to_uppercase(), " ", "    ")
}

/// A panel with the site's `clip-notch` shape: the top-right and bottom-left
/// corners cut at 45 degrees. Paints fill and a 1px outline behind its children.
pub fn notched(notch: f32, fill: Hsla, stroke: Hsla) -> Div {
    div().relative().child(
        canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                let o = bounds.origin;
                let (w, h) = (bounds.size.width, bounds.size.height);
                let n = px(notch);
                let corners = [
                    point(o.x, o.y),
                    point(o.x + w - n, o.y),
                    point(o.x + w, o.y + n),
                    point(o.x + w, o.y + h),
                    point(o.x + n, o.y + h),
                    point(o.x, o.y + h - n),
                ];

                let mut shape = PathBuilder::fill();
                shape.move_to(corners[0]);
                for c in &corners[1..] {
                    shape.line_to(*c);
                }
                shape.close();
                if let Ok(path) = shape.build() {
                    window.paint_path(path, fill);
                }

                // Inset the outline by half a pixel so it isn't clipped.
                let i = px(0.5);
                let inner = [
                    point(corners[0].x + i, corners[0].y + i),
                    point(corners[1].x, corners[1].y + i),
                    point(corners[2].x - i, corners[2].y),
                    point(corners[3].x - i, corners[3].y - i),
                    point(corners[4].x, corners[4].y - i),
                    point(corners[5].x + i, corners[5].y),
                ];
                let mut outline = PathBuilder::stroke(px(1.));
                outline.move_to(inner[0]);
                for c in &inner[1..] {
                    outline.line_to(*c);
                }
                outline.close();
                if let Ok(path) = outline.build() {
                    window.paint_path(path, stroke);
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full(),
    )
}

#[cfg(test)]
mod tests {
    use super::{label_text, tracked, wide_heading_text};

    #[test]
    fn tracks_letters_and_words() {
        assert_eq!(tracked("AB CD", ".", "|"), "A.B|C.D");
        assert_eq!(tracked("  A  ", ".", "|"), "A");
        assert_eq!(wide_heading_text("Hi yo"), "H I    Y O");
        assert_eq!(label_text("ok"), "O\u{200A}K");
    }
}
