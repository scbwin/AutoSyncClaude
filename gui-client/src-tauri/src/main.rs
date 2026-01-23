// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::panic;

fn main() {
    // 设置 panic hook，在 panic 时记录日志
    panic::set_hook(Box::new(|panic_info| {
        let location = panic_info.location().unwrap_or_else(|| panic::Location::caller());
        let message = panic_info.payload().downcast_ref::<String>()
            .map(|s| s.as_str())
            .or_else(|| panic_info.payload().downcast_ref::<&str>().copied())
            .unwrap_or("<unknown panic>");

        eprintln!("!!! PANIC !!!");
        eprintln!("Location: {}", location);
        eprintln!("Message: {}", message);

        // 尝试写入日志文件
        if let Ok(log_dir) = std::env::var("LOCALAPPDATA") {
            let log_path = std::path::PathBuf::from(log_dir)
                .join("claude-sync")
                .join("logs")
                .join("panic.log");
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
            {
                use std::io::Write;
                let _ = writeln!(file, "[{}] PANIC: {} at {} - {}",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                    message,
                    location,
                    std::backtrace::Backtrace::capture()
                );
            }
        }
    }));

    claude_sync_gui::run()
}
