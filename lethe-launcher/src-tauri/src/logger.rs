use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use std::time::SystemTime;

static LOG_LOCK: Mutex<()> = Mutex::new(());
const LOG_PATH: &str = "lethe-launcher.log";

fn format_timestamp() -> String {
    let now = SystemTime::now();
    let duration = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let hours = (secs / 3600) % 24;
    let minutes = (secs / 60) % 60;
    let seconds = secs % 60;
    format!("[{:02}:{:02}:{:02}]", hours, minutes, seconds)
}

pub fn init_logger() {
    let _guard = LOG_LOCK.lock().unwrap();
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_PATH)
    {
        let ts = format_timestamp();
        let line = format!("=== Lethe Launcher Started at {} ===\n", ts);
        let _ = file.write_all(line.as_bytes());
    }
    println!("=== Lethe Launcher Started ===");
}

pub fn log(msg: &str) {
    let _guard = LOG_LOCK.lock().unwrap();
    let ts = format_timestamp();
    let line = format!("{} {}\n", ts, msg);
    print!("{}", line);
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_PATH)
    {
        let _ = file.write_all(line.as_bytes());
    }
}

#[macro_export]
macro_rules! lethe_log {
    ($($arg:tt)*) => {
        $crate::logger::log(&format!($($arg)*))
    };
}
