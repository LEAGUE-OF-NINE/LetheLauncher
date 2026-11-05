use crate::backend::{
    self,
    auth::AuthResult,
    mods::ModInfo,
    sync::{Reporter, SyncEvent, SyncSummary},
    updater::UpdateInfo,
};
use crate::ui::news::NewsCard;
use crate::ui::panels::{sheet_title, ModsPanel, SettingsPanel};
use crate::ui::theme::{self, black_a, white_a};
use crate::{LaunchRequest, LAUNCH_REQUEST};
use gpui_kit::component::button::{Button, ButtonCustomVariant, ButtonVariants as _};
use gpui_kit::component::progress::Progress;
use gpui_kit::component::{
    h_flex, v_flex, Disableable as _, Icon, IconName, Sizable as _, WindowExt as _,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub enum Login {
    Idle,
    Waiting,
    Failed(String),
}

/// The kind of progress event that arrived last; decides what the status
/// line shows.
#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub enum Phase {
    #[default]
    Status,
    Checking,
    Downloading,
    Copying,
}

#[derive(Default, Clone)]
pub struct SyncProgress {
    pub status: String,
    pub phase: Phase,
    pub files_checked: u64,
    pub total_files: u64,
    pub bytes_done: u64,
    pub total_bytes: u64,
    pub current_file: String,
    /// Smoothed download speed in bytes per second.
    pub speed: Option<f64>,
    last_sample: Option<(Instant, u64)>,
}

impl SyncProgress {
    fn percent(&self) -> f32 {
        if self.total_bytes == 0 {
            0.
        } else {
            (self.bytes_done as f64 / self.total_bytes as f64 * 100.) as f32
        }
    }

    fn apply(&mut self, event: SyncEvent) {
        match event {
            SyncEvent::Status(message) => {
                self.status = message;
                self.phase = Phase::Status;
            }
            SyncEvent::Checking {
                files_checked,
                total_files,
                bytes_done,
                total_bytes,
                current_file,
            } => {
                self.phase = Phase::Checking;
                self.files_checked = files_checked;
                self.total_files = total_files;
                self.bytes_done = bytes_done;
                self.total_bytes = total_bytes;
                self.current_file = current_file;
            }
            SyncEvent::Downloading {
                bytes_done,
                total_bytes,
                current_file,
            } => {
                self.phase = Phase::Downloading;
                self.bytes_done = bytes_done;
                self.total_bytes = total_bytes;
                self.current_file = current_file;
                self.sample_speed(bytes_done);
            }
            SyncEvent::Copied {
                bytes_done,
                total_bytes,
                current_file,
            } => {
                self.phase = Phase::Copying;
                self.bytes_done = bytes_done;
                self.total_bytes = total_bytes;
                self.current_file = current_file;
                // Local copies aren't network traffic: restart the speed
                // sample after them so they don't show up as download speed.
                self.last_sample = Some((Instant::now(), bytes_done));
            }
        }
    }

    fn sample_speed(&mut self, bytes_done: u64) {
        let now = Instant::now();
        match self.last_sample {
            None => self.last_sample = Some((now, bytes_done)),
            Some((at, bytes)) => {
                let elapsed = now.duration_since(at).as_secs_f64();
                if elapsed >= 0.5 {
                    let instant = bytes_done.saturating_sub(bytes) as f64 / elapsed;
                    self.speed = Some(match self.speed {
                        Some(prev) => prev * 0.6 + instant * 0.4,
                        None => instant,
                    });
                    self.last_sample = Some((now, bytes_done));
                }
            }
        }
    }
}

pub enum SyncState {
    Idle,
    Running(SyncProgress),
    Done(SyncSummary),
    Failed { message: String, progress: f32 },
}

pub enum UpdateState {
    None,
    Available(UpdateInfo),
    Installing(UpdateInfo),
    Failed(UpdateInfo, String),
}

pub struct LauncherView {
    pub auth: Option<AuthResult>,
    pub avatar: Option<Arc<Image>>,
    pub mods: Vec<ModInfo>,
    pub skip_validation: bool,
    login: Login,
    sync: SyncState,
    launching: bool,
    launch_error: Option<String>,
    update: UpdateState,
    background: Arc<Image>,
    logo: Arc<Image>,
    news: Entity<NewsCard>,
    mods_panel: Option<Entity<ModsPanel>>,
    settings_panel: Option<Entity<SettingsPanel>>,
    login_task: Option<Task<()>>,
    sync_tasks: Vec<Task<()>>,
    _update_task: Option<Task<()>>,
    _avatar_task: Option<Task<()>>,
}

impl LauncherView {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            auth: None,
            avatar: None,
            mods: backend::mods::list_mods(),
            skip_validation: backend::config::is_auto_update_disabled(),
            login: Login::Idle,
            sync: SyncState::Idle,
            launching: false,
            launch_error: None,
            update: UpdateState::None,
            background: super::background_image(),
            logo: super::logo_image(),
            news: cx.new(NewsCard::new),
            mods_panel: None,
            settings_panel: None,
            login_task: None,
            sync_tasks: Vec::new(),
            _update_task: None,
            _avatar_task: None,
        };
        #[cfg(debug_assertions)]
        if let Ok(demo) = std::env::var("LETHE_DEMO") {
            this.apply_demo(&demo, cx);
            return this;
        }

        this.check_for_update(cx);
        if let Some(auth) = backend::auth::load_saved_auth() {
            this.signed_in(auth, cx);
        }
        this
    }

    /// Debug builds only: `LETHE_DEMO=syncing|ready|error|update|login` shows
    /// that state with fake data and no network, for working on the look.
    #[cfg(debug_assertions)]
    fn apply_demo(&mut self, demo: &str, cx: &mut Context<Self>) {
        self.news.update(cx, |news, cx| news.load_demo(cx));
        self.mods = ["BetterUI", "DebugOverlay", "EnglishTL", "FastBattle", "OldPortraits"]
            .iter()
            .enumerate()
            .map(|(i, name)| ModInfo { name: name.to_string(), enabled: i % 3 != 1 })
            .collect();
        if demo == "login" {
            self.login = Login::Waiting;
            return;
        }
        self.auth = Some(AuthResult {
            username: "limi".into(),
            token: String::new(),
            avatar_url: String::new(),
        });
        let mut progress = SyncProgress {
            status: "Downloading 2,308 files...".into(),
            phase: Phase::Downloading,
            files_checked: 3512,
            total_files: 3512,
            bytes_done: 1_870_000_000,
            total_bytes: 4_450_000_000,
            current_file: "LimbusCompany_Data/StreamingAssets/ui_atlas.bundle".into(),
            speed: Some(18_400_000.),
            last_sample: None,
        };
        self.sync = match demo {
            "syncing" => SyncState::Running(progress),
            "checking" => {
                progress.phase = Phase::Checking;
                progress.files_checked = 1204;
                progress.bytes_done = 1_530_000_000;
                SyncState::Running(progress)
            }
            "error" => SyncState::Failed {
                message: "Failed to download manifest: couldn't reach files.lethelc.site".into(),
                progress: 18.,
            },
            _ => SyncState::Done(SyncSummary { total_files: 3512, downloaded_files: 0 }),
        };
        if demo == "update" {
            self.update = UpdateState::Available(UpdateInfo {
                version: "0.2.0".into(),
                notes: String::new(),
                download_url: String::new(),
                sha256: String::new(),
            });
        }
    }

    // ---- account -------------------------------------------------------

    fn start_login(&mut self, cx: &mut Context<Self>) {
        self.login = Login::Waiting;
        self.login_task = Some(cx.spawn(async move |this, cx| {
            let result = backend::run(backend::auth::start_oauth_flow())
                .await
                .and_then(|r| r);
            let _ = this.update(cx, |this, cx| {
                this.login_task = None;
                match result {
                    Ok(auth) => {
                        backend::auth::save_auth(&auth);
                        this.login = Login::Idle;
                        this.signed_in(auth, cx);
                    }
                    Err(e) => {
                        crate::lethe_log!("Login failed: {}", e);
                        this.login = Login::Failed(e);
                    }
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn cancel_login(&mut self, cx: &mut Context<Self>) {
        // Dropping the task stops the polling loop on the backend runtime.
        self.login_task = None;
        self.login = Login::Idle;
        cx.notify();
    }

    fn signed_in(&mut self, auth: AuthResult, cx: &mut Context<Self>) {
        let avatar_url = auth.avatar_url.clone();
        self.auth = Some(auth);
        if !avatar_url.is_empty() {
            self._avatar_task = Some(cx.spawn(async move |this, cx| {
                let bytes = backend::run(async move { backend::fetch_bytes(&avatar_url).await })
                    .await
                    .and_then(|r| r);
                if let Ok(image) = bytes.map(super::image_from_bytes) {
                    let _ = this.update(cx, |this, cx| {
                        this.avatar = image;
                        cx.notify();
                    });
                }
            }));
        }
        if !self.skip_validation {
            self.start_sync(cx);
        }
    }

    pub fn sign_out(&mut self, cx: &mut Context<Self>) {
        backend::auth::clear_saved_auth();
        self.auth = None;
        self.avatar = None;
        self._avatar_task = None;
        self.sync_tasks.clear();
        self.sync = SyncState::Idle;
        self.launch_error = None;
        cx.notify();
    }

    // ---- file sync -----------------------------------------------------

    fn start_sync(&mut self, cx: &mut Context<Self>) {
        let (reporter, events) = Reporter::new();
        self.launch_error = None;
        self.sync = SyncState::Running(SyncProgress {
            status: "Connecting...".into(),
            ..Default::default()
        });

        // Pump progress events into the view, at most ~20 times a second.
        let pump = cx.spawn(async move |this, cx| {
            while let Ok(first) = events.recv().await {
                let mut batch = vec![first];
                while let Ok(more) = events.try_recv() {
                    batch.push(more);
                }
                let alive = this.update(cx, |this, cx| {
                    if let SyncState::Running(progress) = &mut this.sync {
                        for event in batch {
                            progress.apply(event);
                        }
                        cx.notify();
                    }
                });
                if alive.is_err() {
                    break;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
            }
        });

        let job = cx.spawn(async move |this, cx| {
            let result = backend::run(backend::sync::run_sync(reporter))
                .await
                .and_then(|r| r);
            let _ = this.update(cx, |this, cx| {
                this.sync = match result {
                    Ok(summary) => SyncState::Done(summary),
                    Err(message) => {
                        crate::lethe_log!("Sync failed: {}", message);
                        let progress = match &this.sync {
                            SyncState::Running(p) => p.percent(),
                            _ => 0.,
                        };
                        SyncState::Failed { message, progress }
                    }
                };
                cx.notify();
            });
        });

        self.sync_tasks = vec![pump, job];
        cx.notify();
    }

    pub fn set_validation(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.skip_validation = !enabled;
        backend::config::set_config_value("DisableAutoUpdate", if enabled { "false" } else { "true" });
        if enabled {
            if self.auth.is_some() && matches!(self.sync, SyncState::Idle) {
                self.start_sync(cx);
            }
        } else if !matches!(self.sync, SyncState::Done(_)) {
            // The settings promise that Start now launches right away. Stopping
            // midway is safe: downloads only replace files once complete.
            self.sync_tasks.clear();
            self.sync = SyncState::Idle;
        }
        cx.notify();
    }

    // ---- launching -----------------------------------------------------

    fn launch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(self.update, UpdateState::Installing(_)) {
            return;
        }
        self.launch_error = None;
        if let Err(e) = backend::launcher::preflight() {
            crate::lethe_log!("Cannot launch: {}", e);
            self.launch_error = Some(e);
            cx.notify();
            return;
        }
        crate::lethe_log!("Launching game");
        self.launching = true;
        self.sync_tasks.clear();
        *LAUNCH_REQUEST.lock().unwrap() = Some(LaunchRequest {
            token: self.auth.as_ref().map(|a| a.token.clone()),
        });
        cx.notify();
        // Close the window rather than calling cx.quit(): it has to be torn
        // down while the event loop is still running. After a quit, GPUI only
        // queues the window's hide and destroy, nothing runs them, and the
        // window stays on screen "Not Responding" while the game runs. Closing
        // the last window quits the app; main() then starts the game.
        window.remove_window();
    }

    // ---- self-update ---------------------------------------------------

    fn check_for_update(&mut self, cx: &mut Context<Self>) {
        self._update_task = Some(cx.spawn(async move |this, cx| {
            let result = backend::run(backend::updater::check_for_updates(env!(
                "CARGO_PKG_VERSION"
            )))
            .await
            .and_then(|r| r);
            match result {
                Ok(Some(info)) => {
                    let _ = this.update(cx, |this, cx| {
                        this.update = UpdateState::Available(info);
                        cx.notify();
                    });
                }
                Ok(None) => {}
                Err(e) => crate::lethe_log!("Update check failed: {}", e),
            }
        }));
    }

    fn install_update(&mut self, info: UpdateInfo, cx: &mut Context<Self>) {
        self.update = UpdateState::Installing(info.clone());
        self._update_task = Some(cx.spawn(async move |this, cx| {
            let job_info = info.clone();
            // On success the updater exits the process to swap the exe.
            let result = backend::run(async move { backend::updater::download_update(&job_info).await })
                .await
                .and_then(|r| r);
            if let Err(e) = result {
                crate::lethe_log!("Update failed: {}", e);
                let _ = this.update(cx, |this, cx| {
                    this.update = UpdateState::Failed(info, e);
                    cx.notify();
                });
            }
        }));
        cx.notify();
    }

    // ---- mods ----------------------------------------------------------

    pub fn refresh_mods(&mut self, cx: &mut Context<Self>) {
        self.mods = backend::mods::list_mods();
        cx.notify();
    }

    pub fn toggle_mod(&mut self, name: &str, cx: &mut Context<Self>) {
        match backend::mods::toggle_mod(name) {
            Ok(updated) => {
                if let Some(m) = self.mods.iter_mut().find(|m| m.name == updated.name) {
                    m.enabled = updated.enabled;
                }
            }
            Err(e) => {
                crate::lethe_log!("{}", e);
                self.mods = backend::mods::list_mods();
            }
        }
        cx.notify();
    }

    pub fn set_all_mods(&mut self, enabled: bool, cx: &mut Context<Self>) {
        let names: Vec<String> = self
            .mods
            .iter()
            .filter(|m| m.enabled != enabled)
            .map(|m| m.name.clone())
            .collect();
        for name in names {
            if let Err(e) = backend::mods::toggle_mod(&name) {
                crate::lethe_log!("{}", e);
            }
        }
        self.refresh_mods(cx);
    }

    // ---- panels --------------------------------------------------------

    fn open_mods(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_mods(cx);
        let launcher = cx.entity().downgrade();
        let panel = self
            .mods_panel
            .get_or_insert_with(|| cx.new(|cx| ModsPanel::new(launcher, cx)))
            .clone();
        window.open_sheet(cx, move |sheet, _, _| {
            sheet.title(sheet_title("Mods")).size(px(380.)).resizable(false).child(panel.clone())
        });
    }

    fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let launcher = cx.entity().downgrade();
        let panel = self
            .settings_panel
            .get_or_insert_with(|| cx.new(|cx| SettingsPanel::new(launcher, cx)))
            .clone();
        window.open_sheet(cx, move |sheet, _, _| {
            sheet
                .title(sheet_title("Settings"))
                .size(px(380.))
                .resizable(false)
                .child(panel.clone())
        });
    }

    // ---- rendering -----------------------------------------------------

    fn render_title_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let control = |id: &'static str, icon: IconName, area: WindowControlArea, hover: Hsla| {
            div()
                .id(id)
                .w(px(46.))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(white_a(0.65))
                .hover(move |s| s.bg(hover).text_color(white_a(1.)))
                .window_control_area(area)
                .child(Icon::new(icon).small())
        };

        h_flex()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .h(px(36.))
            .child(
                h_flex()
                    .id("drag")
                    .flex_1()
                    .h_full()
                    .pl(px(14.))
                    .gap(px(10.))
                    .window_control_area(WindowControlArea::Drag)
                    .child(img(self.logo.clone()).size(px(20.)))
                    .child(
                        div()
                            .font_family(theme::HEADING_FONT)
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_size(px(16.))
                            .text_color(theme::foreground())
                            .child(theme::wide_heading_text("Lethe")),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(white_a(0.35))
                            .child(concat!("v", env!("CARGO_PKG_VERSION"))),
                    ),
            )
            .when_some(self.auth.as_ref(), |bar, auth| {
                bar.child(
                    h_flex()
                        .id("account")
                        .h_full()
                        .px(px(10.))
                        .gap(px(8.))
                        .cursor_pointer()
                        .hover(|s| s.bg(white_a(0.06)))
                        .on_click(cx.listener(|this, _, window, cx| this.open_settings(window, cx)))
                        .child(avatar(self.avatar.clone(), &auth.username, 22.))
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(white_a(0.85))
                                .child(auth.username.clone()),
                        ),
                )
            })
            .child(control(
                "minimize",
                IconName::WindowMinimize,
                WindowControlArea::Min,
                white_a(0.1),
            ))
            .child(control(
                "close",
                IconName::WindowClose,
                WindowControlArea::Close,
                theme::primary(),
            ))
    }

    /// "WELCOME TO / LETHE" block, as on the site's landing page.
    fn render_hero(&self) -> impl IntoElement {
        v_flex()
            .absolute()
            .left(px(56.))
            .top(px(92.))
            .child(
                div()
                    .font_family(theme::HEADING_FONT)
                    .font_weight(FontWeight::MEDIUM)
                    .text_size(px(22.))
                    .text_color(white_a(0.7))
                    .child(theme::wide_heading_text("Welcome to")),
            )
            .child(
                div()
                    .font_family(theme::HEADING_FONT)
                    .font_weight(FontWeight::BOLD)
                    .text_size(px(104.))
                    .line_height(px(100.))
                    .text_color(theme::primary())
                    .child("LETHE"),
            )
            .child(div().mt(px(10.)).w(px(96.)).h(px(2.)).bg(theme::primary()))
            .child(
                div()
                    .mt(px(18.))
                    .max_w(px(520.))
                    .text_size(px(18.))
                    .text_color(white_a(0.85))
                    .child("A private server and modding hub for Limbus Company"),
            )
    }

    fn render_update_notice(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let (text, action): (String, Option<AnyElement>) = match &self.update {
            UpdateState::None => return None,
            UpdateState::Available(info) => {
                let info = info.clone();
                (
                    format!("Launcher v{} is available", info.version),
                    Some(
                        Button::new("install-update")
                            .primary()
                            .xsmall()
                            .label(theme::label_text("Update"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.install_update(info.clone(), cx)
                            }))
                            .into_any_element(),
                    ),
                )
            }
            UpdateState::Installing(info) => (
                format!("Installing v{}... the launcher will restart", info.version),
                Some(Button::new("installing").ghost().xsmall().loading(true).into_any_element()),
            ),
            UpdateState::Failed(info, e) => {
                let info = info.clone();
                (
                    format!("Update failed: {}", e),
                    Some(
                        Button::new("retry-update")
                            .outline()
                            .xsmall()
                            .label(theme::label_text("Retry"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.install_update(info.clone(), cx)
                            }))
                            .into_any_element(),
                    ),
                )
            }
        };
        let dismissable = !matches!(self.update, UpdateState::Installing(_));

        Some(
            h_flex()
                .absolute()
                .top(px(48.))
                .right(px(16.))
                .max_w(px(460.))
                .gap(px(12.))
                .pl(px(12.))
                .pr(px(6.))
                .py(px(6.))
                .bg(theme::card().opacity(0.95))
                .border_l_2()
                .border_color(theme::gold())
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme::gold())
                        .child(theme::label_text("Update")),
                )
                .child(
                    div()
                        .flex_1()
                        .text_size(px(12.))
                        .text_color(white_a(0.85))
                        .child(text),
                )
                .children(action)
                .when(dismissable, |row| {
                    row.child(
                        Button::new("dismiss-update")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Close)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.update = UpdateState::None;
                                cx.notify();
                            })),
                    )
                }),
        )
    }

    fn render_actions(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let signed_in = self.auth.is_some();
        // Launching quits the launcher, which would cut a self-update off midway.
        let installing = matches!(self.update, UpdateState::Installing(_));

        let mut status: Status;
        let mut bar: Option<(f32, Hsla, bool)> = None;
        let mut link: Option<(&'static str, &'static str)> = None;

        let primary: AnyElement = if !signed_in {
            let waiting = matches!(self.login, Login::Waiting);
            status = match &self.login {
                Login::Idle => Status::Text("Sign in to check files and play".into()),
                Login::Waiting => {
                    link = Some(("cancel-login", "Cancel"));
                    Status::Text("Finish signing in in your browser".into())
                }
                Login::Failed(e) => Status::Bad(e.clone().into()),
            };
            big_button(
                "sign-in",
                if waiting { "Waiting for browser..." } else { "Sign in with Discord" },
            )
            .primary()
            .loading(waiting)
            .disabled(waiting)
            .on_click(cx.listener(|this, _, _, cx| this.start_login(cx)))
            .into_any_element()
        } else if self.launching {
            status = Status::Text("Starting Limbus Company...".into());
            big_button("launching", "Launching...")
                .primary()
                .loading(true)
                .disabled(true)
                .into_any_element()
        } else {
            match &self.sync {
                SyncState::Running(p) => {
                    let left: SharedString = match p.phase {
                        Phase::Downloading => {
                            format!("Downloading {}", short_path(&p.current_file)).into()
                        }
                        Phase::Copying => {
                            format!("Copying {} from Steam", short_path(&p.current_file)).into()
                        }
                        Phase::Checking => format!(
                            "Checking files {} / {}",
                            fmt_count(p.files_checked),
                            fmt_count(p.total_files)
                        )
                        .into(),
                        Phase::Status => p.status.clone().into(),
                    };
                    let right = (p.total_bytes > 0).then(|| {
                        let mut r = format!("{:.0}%", p.percent());
                        if let (Phase::Downloading, Some(speed)) = (p.phase, p.speed) {
                            r.push_str(&format!("  ·  {}/s", backend::format_bytes(speed as u64)));
                        }
                        SharedString::from(r)
                    });
                    status = Status::Progress(left, right);
                    bar = Some((p.percent(), theme::primary(), p.total_bytes == 0));
                    link = (!installing).then_some(("skip-launch", "Skip & launch"));
                    big_button("updating", "Updating...")
                        .custom(flat_variant(theme::secondary(), theme::muted_foreground(), cx))
                        .bg(theme::secondary())
                        .disabled(true)
                        .into_any_element()
                }
                SyncState::Failed { message, progress } => {
                    status = Status::Bad(message.clone().into());
                    bar = Some((*progress, white_a(0.3), false));
                    link = (!installing).then_some(("launch-anyway", "Launch anyway"));
                    big_button("retry", "Retry")
                        .custom(
                            ButtonCustomVariant::new(cx)
                                .color(black_a(0.5))
                                .foreground(theme::foreground())
                                .hover(white_a(0.08))
                                .active(white_a(0.14)),
                        )
                        .bg(black_a(0.5))
                        .border_1()
                        .border_color(white_a(0.3))
                        .on_click(cx.listener(|this, _, _, cx| this.start_sync(cx)))
                        .into_any_element()
                }
                SyncState::Done(summary) => {
                    status = Status::Good(if summary.downloaded_files > 0 {
                        format!(
                            "Updated {} files · all {} verified",
                            fmt_count(summary.downloaded_files),
                            fmt_count(summary.total_files)
                        )
                        .into()
                    } else {
                        format!("All {} files verified", fmt_count(summary.total_files)).into()
                    });
                    big_button("start", "Start game")
                        .primary()
                        .disabled(installing)
                        .on_click(cx.listener(|this, _, window, cx| this.launch(window, cx)))
                        .into_any_element()
                }
                SyncState::Idle => {
                    status = Status::Text("File check is off".into());
                    big_button("start", "Start game")
                        .primary()
                        .disabled(installing)
                        .on_click(cx.listener(|this, _, window, cx| this.launch(window, cx)))
                        .into_any_element()
                }
            }
        };

        if installing && signed_in && !self.launching {
            status = Status::Text("Updating the launcher. It will restart when done".into());
        }
        if let Some(e) = &self.launch_error {
            status = Status::Bad(e.clone().into());
        }

        let link_el = link.map(|(id, label)| {
            div()
                .id(id)
                .mr_auto()
                .text_size(px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme::gold())
                .cursor_pointer()
                .hover(|s| s.text_color(theme::gold().opacity(0.8)))
                .child(format!("{}  \u{2192}", theme::label_text(label)))
                .on_click(cx.listener(move |this, _, window, cx| match id {
                    "cancel-login" => this.cancel_login(cx),
                    _ => this.launch(window, cx),
                }))
        });

        v_flex()
            .absolute()
            .right(px(28.))
            .bottom(px(28.))
            .w(px(440.))
            .gap(px(10.))
            .child(status.render())
            .when_some(bar, |col, (value, color, indeterminate)| {
                col.child(
                    Progress::new("sync-progress")
                        .xsmall()
                        .color(color)
                        .loading(indeterminate)
                        .value(value),
                )
            })
            .child(
                h_flex()
                    .w_full()
                    .gap(px(8.))
                    .justify_end()
                    .items_center()
                    .children(link_el)
                    .child(
                        icon_button("settings", IconName::Settings, "Settings", cx)
                            .on_click(cx.listener(|this, _, window, cx| this.open_settings(window, cx))),
                    )
                    .child(
                        icon_button("mods", gpui_kit::assets::IconName::Puzzle, "Mods", cx)
                            .on_click(cx.listener(|this, _, window, cx| this.open_mods(window, cx))),
                    )
                    .child(primary),
            )
    }
}

/// The line above the progress bar.
enum Status {
    /// Plain muted note, right-aligned.
    Text(SharedString),
    /// What's happening on the left, percent/speed on the right.
    Progress(SharedString, Option<SharedString>),
    /// Gold badge, like the site's "Implemented" tags.
    Good(SharedString),
    /// Red badge for errors.
    Bad(SharedString),
}

impl Status {
    fn render(self) -> AnyElement {
        let badge = |text: SharedString, icon: IconName, color: Hsla| {
            h_flex().w_full().justify_end().child(
                h_flex()
                    .max_w_full()
                    .gap(px(6.))
                    .px(px(8.))
                    .py(px(3.))
                    .bg(color.opacity(0.14))
                    .text_color(color)
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(Icon::new(icon).xsmall())
                    .child(div().min_w_0().truncate().child(text)),
            )
        };
        match self {
            Status::Text(text) => div()
                .w_full()
                .text_right()
                .text_size(px(12.))
                .text_color(theme::muted_foreground())
                .child(text)
                .into_any_element(),
            Status::Progress(left, right) => h_flex()
                .w_full()
                .gap(px(12.))
                .justify_between()
                .text_size(px(12.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_color(theme::muted_foreground())
                        .child(left),
                )
                .children(right.map(|r| {
                    div()
                        .flex_none()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme::foreground())
                        .child(r)
                }))
                .into_any_element(),
            Status::Good(text) => badge(text, IconName::Check, theme::gold()).into_any_element(),
            Status::Bad(text) => {
                badge(text, IconName::TriangleAlert, theme::destructive()).into_any_element()
            }
        }
    }
}

impl Render for LauncherView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let scrim = |angle: f32, from: f32, to: f32| {
            div().absolute().top_0().left_0().size_full().bg(linear_gradient(
                angle,
                linear_color_stop(black_a(from), 0.),
                linear_color_stop(black_a(0.), to),
            ))
        };
        div()
            .size_full()
            .relative()
            .overflow_hidden()
            .bg(theme::background())
            .text_color(theme::foreground())
            .child(
                img(self.background.clone())
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .object_fit(ObjectFit::Cover),
            )
            // The art is composed dark where the UI sits; these only add a
            // little extra weight behind the title, news and controls.
            .child(scrim(90., 0.35, 0.45))
            .child(scrim(0., 0.5, 0.3))
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(px(36.))
                    .bg(theme::sidebar().opacity(0.6))
                    .border_b_1()
                    .border_color(white_a(0.06)),
            )
            .child(self.render_hero())
            .child(
                div()
                    .absolute()
                    .left(px(56.))
                    .bottom(px(28.))
                    .w(px(380.))
                    .child(self.news.clone()),
            )
            .child(self.render_actions(cx))
            .children(self.render_update_notice(cx))
            .child(self.render_title_bar(cx))
    }
}

// ---- small building blocks ----------------------------------------------

/// The site's buttons: square, uppercase. The big ones use the heading font.
fn big_button(id: &'static str, label: &'static str) -> Button {
    Button::new(id)
        .large()
        .h(px(52.))
        .px(px(34.))
        .child(
            div()
                .font_family(theme::HEADING_FONT)
                .font_weight(FontWeight::BOLD)
                .text_size(px(21.))
                .child(label.to_uppercase()),
        )
}

fn flat_variant(bg: Hsla, fg: Hsla, cx: &App) -> ButtonCustomVariant {
    ButtonCustomVariant::new(cx)
        .color(bg)
        .foreground(fg)
        .hover(bg)
        .active(bg)
}

fn icon_button(
    id: &'static str,
    icon: impl Into<Icon>,
    tooltip: &'static str,
    cx: &App,
) -> Button {
    Button::new(id)
        .icon(icon.into())
        .tooltip(tooltip)
        .w(px(52.))
        .h(px(52.))
        .custom(
            ButtonCustomVariant::new(cx)
                .color(black_a(0.55))
                .foreground(white_a(0.8))
                .hover(white_a(0.1))
                .active(white_a(0.16)),
        )
        .bg(black_a(0.55))
        .border_1()
        .border_color(theme::border())
}

/// Round avatar, or the first letter of the name until the image arrives.
pub fn avatar(image: Option<Arc<Image>>, name: &str, size: f32) -> AnyElement {
    match image {
        Some(image) => img(image)
            .size(px(size))
            .rounded_full()
            .object_fit(ObjectFit::Cover)
            .into_any_element(),
        None => div()
            .size(px(size))
            .rounded_full()
            .bg(theme::primary())
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(size * 0.5))
            .font_weight(FontWeight::BOLD)
            .text_color(white())
            .child(
                name.chars()
                    .next()
                    .map(|c| c.to_uppercase().to_string())
                    .unwrap_or_default(),
            )
            .into_any_element(),
    }
}

fn short_path(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

fn fmt_count(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{fmt_count, short_path, Phase, SyncEvent, SyncProgress};

    #[test]
    fn formats_counts_with_commas() {
        assert_eq!(fmt_count(0), "0");
        assert_eq!(fmt_count(999), "999");
        assert_eq!(fmt_count(1204), "1,204");
        assert_eq!(fmt_count(3512000), "3,512,000");
    }

    #[test]
    fn short_path_keeps_file_name() {
        assert_eq!(short_path("Assets/ui/atlas.bundle"), "atlas.bundle");
        assert_eq!(short_path("plain.dll"), "plain.dll");
    }

    #[test]
    fn progress_tracks_events() {
        let mut p = SyncProgress::default();
        p.apply(SyncEvent::Checking {
            files_checked: 1,
            total_files: 4,
            bytes_done: 25,
            total_bytes: 100,
            current_file: "a".into(),
        });
        assert_eq!(p.phase, Phase::Checking);
        assert_eq!(p.percent(), 25.);
        p.apply(SyncEvent::Downloading {
            bytes_done: 50,
            total_bytes: 100,
            current_file: "b".into(),
        });
        assert_eq!(p.phase, Phase::Downloading);
        assert_eq!(p.percent(), 50.);
        assert_eq!(p.current_file, "b");

        p.apply(SyncEvent::Copied {
            bytes_done: 90,
            total_bytes: 100,
            current_file: "c".into(),
        });
        assert_eq!(p.phase, Phase::Copying);
        // The copy restarts the speed sample instead of counting as download.
        assert_eq!(p.last_sample.map(|(_, bytes)| bytes), Some(90));

        p.apply(SyncEvent::Status("Downloading Lethe.dll...".into()));
        assert_eq!(p.phase, Phase::Status);
        assert_eq!(p.status, "Downloading Lethe.dll...");
    }
}
