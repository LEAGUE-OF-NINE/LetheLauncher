mod launcher;
mod news;
mod panels;
pub mod theme;

use gpui_kit::*;
use std::borrow::Cow;
use std::sync::Arc;

gpui_kit::assets::icon_assets!(ExtraIcons, [Puzzle, LogOut, Download, FolderOpen]);

/// Component icons plus the few extra Lucide icons the launcher uses.
struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(bytes) = ExtraIcons.load(path)? {
            return Ok(Some(bytes));
        }
        gpui_kit::assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = gpui_kit::assets::Assets.list(path)?;
        paths.extend(ExtraIcons.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

/// Key art shown behind everything. Replace `assets/background.jpg` to change it.
pub fn background_image() -> Arc<Image> {
    static BYTES: &[u8] = include_bytes!("../../assets/background.jpg");
    Arc::new(Image::from_bytes(ImageFormat::Jpeg, BYTES.to_vec()))
}

/// The Lethe rose, as on lethelc.site.
pub fn logo_image() -> Arc<Image> {
    static BYTES: &[u8] = include_bytes!("../../assets/logo.png");
    Arc::new(Image::from_bytes(ImageFormat::Png, BYTES.to_vec()))
}

/// Decode downloaded image bytes by sniffing the format (avatars and banners).
pub fn image_from_bytes(bytes: Vec<u8>) -> Option<Arc<Image>> {
    let format = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        ImageFormat::Png
    } else if bytes.starts_with(&[0xFF, 0xD8]) {
        ImageFormat::Jpeg
    } else if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        ImageFormat::Webp
    } else if bytes.starts_with(b"GIF8") {
        ImageFormat::Gif
    } else {
        return None;
    };
    Some(Arc::new(Image::from_bytes(format, bytes)))
}

/// Gacha-style launchers sit at 1280x720. Shrink (keeping 16:9) on small or
/// heavily scaled displays so the window never fills the whole screen.
fn window_size(cx: &App) -> Size<Pixels> {
    let mut width = 1280.0_f32;
    if let Some(display) = cx.primary_display() {
        let screen = display.bounds().size;
        let max_w = f32::from(screen.width) * 0.85;
        let max_h = f32::from(screen.height) * 0.85;
        width = width.min(max_w).min(max_h * 16.0 / 9.0);
    }
    size(px(width.round()), px((width * 9.0 / 16.0).round()))
}

pub fn run() {
    gpui_kit::application()
        .with_assets(AppAssets)
        .run(|cx| {
            gpui_kit::init(cx);
            theme::load_fonts(cx);
            theme::apply(cx);

            let options = WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some("Lethe Launcher".into()),
                    appears_transparent: true,
                    traffic_light_position: None,
                }),
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    window_size(cx),
                    cx,
                ))),
                is_resizable: false,
                window_decorations: Some(WindowDecorations::Client),
                app_id: Some("lethe-launcher".into()),
                ..Default::default()
            };

            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| launcher::LauncherView::new(window, cx))
            })
            .expect("failed to open launcher window");
            cx.activate(true);
        });
}
