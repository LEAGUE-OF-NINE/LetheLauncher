// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backend;
mod ui;

use std::sync::Mutex;

/// Set by the UI when the player presses Start. The UI then quits, and the
/// game is started from here once the launcher window is fully torn down.
pub static LAUNCH_REQUEST: Mutex<Option<LaunchRequest>> = Mutex::new(None);

pub struct LaunchRequest {
    pub token: Option<String>,
}

fn main() {
    backend::logger::init_logger();
    backend::config::create_default_config();
    if let Ok(cwd) = std::env::current_dir() {
        lethe_log!("Working directory: {}", cwd.display());
    }

    ui::run();

    let request = LAUNCH_REQUEST.lock().unwrap().take();
    if let Some(request) = request {
        // Unity runs on a fresh thread, the same way the previous launcher
        // started it, rather than on the thread GPUI used for its window.
        let handle = std::thread::Builder::new()
            .name("unity-main".into())
            .spawn(move || backend::launcher::launch_game(request.token.as_deref()))
            .expect("failed to spawn game thread");
        let failure = match handle.join() {
            Ok(Ok(())) => None,
            Ok(Err(e)) => Some(e),
            Err(_) => Some("The game thread crashed.".to_string()),
        };
        if let Some(message) = failure {
            lethe_log!("Launch failed: {}", message);
            show_error_box(&format!("Couldn't start the game.\n\n{}", message));
        }
    }
}

/// The launcher window is already gone by the time the game starts, so a
/// failure at that point gets a plain system dialog.
#[cfg(windows)]
fn show_error_box(message: &str) {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(hwnd: *mut c_void, text: *const u16, caption: *const u16, kind: u32) -> i32;
    }
    let wide = |s: &str| -> Vec<u16> {
        std::ffi::OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
    };
    let (text, caption) = (wide(message), wide("Lethe Launcher"));
    const MB_ICONERROR: u32 = 0x10;
    unsafe {
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), caption.as_ptr(), MB_ICONERROR);
    }
}

#[cfg(not(windows))]
fn show_error_box(message: &str) {
    eprintln!("{}", message);
}
