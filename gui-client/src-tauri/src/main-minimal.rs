// 最小化测试版本
#![cfg_attr(debug_assertions, windows_subsystem = "console"))

fn main() {
    println!("=== 最小化 Tauri 测试 ===");
    println!("开始创建窗口...");

    tauri::Builder::default()
        .setup(|app| {
            println!("Setup 函数执行");
            if let Some(window) = app.get_window("main") {
                println!("获取到主窗口");
                match window.show() {
                    Ok(_) => println!("窗口显示成功"),
                    Err(e) => println!("窗口显示失败: {}", e),
                }
            } else {
                println!("无法获取主窗口！");
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    println!("Tauri 应用已退出");
}
