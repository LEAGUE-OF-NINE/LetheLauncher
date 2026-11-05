//! Contents of the Mods and Settings sheets. Both read their state from the
//! launcher view and re-render whenever it changes.

use crate::ui::launcher::{avatar, LauncherView};
use crate::ui::theme::{self, white_a};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _, Icon, IconName, Sizable as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

const MODS_DIR: &str = "BepInEx\\plugins\\Lethe\\mods";

fn open_folder(path: &str) {
    let target = std::env::current_dir()
        .map(|cwd| cwd.join(path))
        .unwrap_or_else(|_| path.into());
    let _ = std::fs::create_dir_all(&target);
    if let Err(e) = std::process::Command::new("explorer").arg(&target).spawn() {
        crate::lethe_log!("Failed to open {}: {}", target.display(), e);
    }
}

/// Sheet header in the heading font, like the site's section titles.
pub fn sheet_title(text: &str) -> impl IntoElement {
    div()
        .font_family(theme::HEADING_FONT)
        .font_weight(FontWeight::SEMIBOLD)
        .text_size(px(22.))
        .text_color(theme::foreground())
        .child(theme::wide_heading_text(text))
}

/// Tracked uppercase label with a red accent bar.
fn section_label(text: &'static str) -> impl IntoElement {
    h_flex()
        .gap(px(8.))
        .child(div().w(px(2.)).h(px(11.)).bg(theme::primary()))
        .child(
            div()
                .text_size(px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme::muted_foreground())
                .child(theme::label_text(text)),
        )
}

// ---- Mods ---------------------------------------------------------------

pub struct ModsPanel {
    launcher: WeakEntity<LauncherView>,
    _observe: Option<Subscription>,
}

impl ModsPanel {
    pub fn new(launcher: WeakEntity<LauncherView>, cx: &mut Context<Self>) -> Self {
        let observe = launcher
            .upgrade()
            .map(|l| cx.observe(&l, |_, _, cx| cx.notify()));
        Self {
            launcher,
            _observe: observe,
        }
    }
}

impl Render for ModsPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(launcher) = self.launcher.upgrade() else {
            return div().into_any_element();
        };
        let mods = launcher.read(cx).mods.clone();
        let enabled = mods.iter().filter(|m| m.enabled).count();
        let border = cx.theme().border;

        v_flex()
            .gap(px(14.))
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme::muted_foreground())
                            .child(format!("{} of {} enabled", enabled, mods.len())),
                    )
                    .child(
                        h_flex()
                            .gap(px(6.))
                            .child(
                                Button::new("enable-all")
                                    .ghost()
                                    .xsmall()
                                    .label(theme::label_text("Enable all"))
                                    .on_click({
                                        let launcher = launcher.clone();
                                        move |_, _, cx| {
                                            launcher.update(cx, |l, cx| l.set_all_mods(true, cx))
                                        }
                                    }),
                            )
                            .child(
                                Button::new("disable-all")
                                    .ghost()
                                    .xsmall()
                                    .label(theme::label_text("Disable all"))
                                    .on_click({
                                        let launcher = launcher.clone();
                                        move |_, _, cx| {
                                            launcher.update(cx, |l, cx| l.set_all_mods(false, cx))
                                        }
                                    }),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .when(mods.is_empty(), |list| {
                        list.child(
                            div()
                                .py(px(24.))
                                .text_size(px(12.))
                                .text_color(white_a(0.45))
                                .child(format!("No mods found in {}", MODS_DIR)),
                        )
                    })
                    .children(mods.into_iter().enumerate().map(|(i, m)| {
                        let name = m.name.clone();
                        let launcher = launcher.clone();
                        h_flex()
                            .py(px(9.))
                            .gap(px(12.))
                            .justify_between()
                            .border_b_1()
                            .border_color(border)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(13.))
                                    .text_color(if m.enabled { theme::foreground() } else { white_a(0.4) })
                                    .child(m.name.clone()),
                            )
                            .child(
                                Switch::new(("mod", i))
                                    .checked(m.enabled)
                                    .on_click(move |_, _, cx| {
                                        launcher.update(cx, |l, cx| l.toggle_mod(&name, cx))
                                    }),
                            )
                    })),
            )
            .child(
                h_flex().child(
                    Button::new("open-mods-folder")
                        .outline()
                        .small()
                        .icon(Icon::new(gpui_kit::assets::IconName::FolderOpen))
                        .label(theme::label_text("Open mods folder"))
                        .on_click(|_, _, _| open_folder(MODS_DIR)),
                ),
            )
            .into_any_element()
    }
}

// ---- Settings -----------------------------------------------------------

pub struct SettingsPanel {
    launcher: WeakEntity<LauncherView>,
    _observe: Option<Subscription>,
}

impl SettingsPanel {
    pub fn new(launcher: WeakEntity<LauncherView>, cx: &mut Context<Self>) -> Self {
        let observe = launcher
            .upgrade()
            .map(|l| cx.observe(&l, |_, _, cx| cx.notify()));
        Self {
            launcher,
            _observe: observe,
        }
    }
}

impl Render for SettingsPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(launcher) = self.launcher.upgrade() else {
            return div().into_any_element();
        };
        let state = launcher.read(cx);
        let validate = !state.skip_validation;
        let auth = state.auth.clone();
        let avatar_image = state.avatar.clone();

        v_flex()
            .gap(px(22.))
            .child(
                v_flex()
                    .gap(px(10.))
                    .child(section_label("Game files"))
                    .child(
                        h_flex()
                            .gap(px(16.))
                            .justify_between()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap(px(2.))
                                    .child(div().text_size(px(13.)).child("Check files on startup"))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(theme::muted_foreground())
                                            .child(if validate {
                                                "Checks every game file and downloads updates before you play."
                                            } else {
                                                "Files aren't checked. Start launches the game immediately."
                                            }),
                                    ),
                            )
                            .child(Switch::new("validate").checked(validate).on_click({
                                let launcher = launcher.clone();
                                move |checked, _, cx| {
                                    let checked = *checked;
                                    launcher.update(cx, |l, cx| l.set_validation(checked, cx))
                                }
                            })),
                    )
                    .child(
                        h_flex().child(
                            Button::new("open-game-folder")
                                .outline()
                                .small()
                                .icon(Icon::new(gpui_kit::assets::IconName::FolderOpen))
                                .label(theme::label_text("Open game folder"))
                                .on_click(|_, _, _| open_folder(".")),
                        ),
                    ),
            )
            .child(
                v_flex()
                    .gap(px(10.))
                    .child(section_label("Account"))
                    .child(match auth {
                        Some(auth) => h_flex()
                            .gap(px(10.))
                            .child(avatar(avatar_image, &auth.username, 32.))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .child(div().text_size(px(13.)).child(auth.username.clone()))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(theme::muted_foreground())
                                            .child("Signed in with Discord"),
                                    ),
                            )
                            .child(
                                Button::new("sign-out")
                                    .outline()
                                    .small()
                                    .icon(Icon::new(gpui_kit::assets::IconName::LogOut))
                                    .label(theme::label_text("Sign out"))
                                    .on_click({
                                        let launcher = launcher.clone();
                                        move |_, _, cx| launcher.update(cx, |l, cx| l.sign_out(cx))
                                    }),
                            )
                            .into_any_element(),
                        None => div()
                            .text_size(px(12.))
                            .text_color(theme::muted_foreground())
                            .child("Not signed in")
                            .into_any_element(),
                    }),
            )
            .child(
                v_flex()
                    .gap(px(6.))
                    .child(section_label("About"))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme::muted_foreground())
                            .child(concat!("Lethe Launcher v", env!("CARGO_PKG_VERSION"))),
                    )
                    .child(
                        h_flex().child(
                            Button::new("open-log")
                                .outline()
                                .small()
                                .icon(Icon::new(IconName::FileText))
                                .label(theme::label_text("Open log file"))
                                .on_click(|_, _, _| {
                                    let _ = std::process::Command::new("notepad")
                                        .arg("lethe-launcher.log")
                                        .spawn();
                                }),
                        ),
                    ),
            )
            .into_any_element()
    }
}
