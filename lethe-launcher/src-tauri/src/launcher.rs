use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

/// Launch Limbus Company by loading UnityPlayer.dll, injecting doorstop, and calling UnityMain.
/// This is Windows-only and mirrors the C# GameStarter.StartGame().
pub fn launch_game(auth_token: Option<&str>) -> Result<(), String> {
    crate::lethe_log!("Starting game...");

    // Set doorstop env var
    std::env::set_var("DOORSTOP_DISABLE_PROXY", "1");

    // Set LETHE_TOKEN for the BepInEx mod to read (avoids browser polling flow)
    if let Some(token) = auth_token {
        std::env::set_var("LETHE_TOKEN", token);
        crate::lethe_log!("LETHE_TOKEN set ({} chars)", token.len());
    }

    let cwd = std::env::current_dir().map_err(|e| format!("Cannot get CWD: {}", e))?;
    crate::lethe_log!("Working directory: {}", cwd.display());

    // Verify required files exist
    let unity_player_path = Path::new("UnityPlayer.dll");
    let doorstop_path = Path::new("BepInEx/doorstop/doorstop.dll");

    if !unity_player_path.exists() {
        return Err(format!(
            "UnityPlayer.dll not found at: {}",
            std::fs::canonicalize(unity_player_path)
                .unwrap_or_else(|_| unity_player_path.to_path_buf())
                .display()
        ));
    }
    crate::lethe_log!("Found UnityPlayer.dll");

    if !doorstop_path.exists() {
        return Err(format!(
            "doorstop.dll not found at: {}",
            std::fs::canonicalize(doorstop_path)
                .unwrap_or_else(|_| doorstop_path.to_path_buf())
                .display()
        ));
    }
    crate::lethe_log!("Found doorstop.dll");

    unsafe {
        // Load UnityPlayer.dll first (required by doorstop)
        crate::lethe_log!("Loading UnityPlayer.dll...");
        let unity_player = libloading::Library::new("UnityPlayer.dll")
            .map_err(|e| format!("Failed to load UnityPlayer.dll: {}", e))?;
        crate::lethe_log!("UnityPlayer.dll loaded successfully");

        // Load doorstop.dll to trigger its DllEntry
        crate::lethe_log!("Loading doorstop.dll...");
        let _doorstop = libloading::Library::new("BepInEx/doorstop/doorstop.dll")
            .map_err(|e| format!("Failed to load doorstop.dll: {}", e))?;
        crate::lethe_log!("doorstop.dll loaded successfully");

        // Get UnityMain function pointer
        // Signature: extern "stdcall" fn(hInstance, hPrevInstance, lpCmdLine, nShowCmd) -> int
        let unity_main: libloading::Symbol<
            unsafe extern "system" fn(isize, isize, *const u16, i32) -> i32,
        > = unity_player
            .get(b"UnityMain")
            .map_err(|e| format!("Failed to find UnityMain: {}", e))?;

        // Create empty wide string for command line argument
        let empty_cmd_line: Vec<u16> = OsStr::new("")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        crate::lethe_log!("Calling UnityMain...");
        let result = unity_main(0, 0, empty_cmd_line.as_ptr(), 1);
        crate::lethe_log!("UnityMain returned: {}", result);
    }

    Ok(())
}
