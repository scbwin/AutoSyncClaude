use std::io::Result;
use std::path::Path;
use std::process::Command;

fn main() -> Result<()> {
    // 获取项目根目录（CARGO_MANIFEST_DIR 指向 server 目录）
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR 环境变量未设置");

    // proto 文件位于 server 的父目录
    let proto_dir = format!("{}/../proto", manifest_dir);
    let proto_file = format!("{}/sync.proto", proto_dir);

    // 规范化路径，解析 ..
    let proto_dir = Path::new(&proto_dir).canonicalize()
        .unwrap_or_else(|e| {
            eprintln!("警告: 无法规范化 proto_dir: {}, 错误: {}", proto_dir, e);
            Path::new(&proto_dir).to_path_buf()
        });
    let proto_file = proto_dir.join("sync.proto");

    // 调试输出
    eprintln!("CARGO_MANIFEST_DIR: {}", manifest_dir);
    eprintln!("proto_dir (canonicalized): {}", proto_dir.display());
    eprintln!("proto_file: {}", proto_file.display());
    eprintln!("proto_file exists: {}", proto_file.exists());

    // 检查 protoc 是否可用
    let protoc_check = Command::new("protoc")
        .arg("--version")
        .output();
    match protoc_check {
        Ok(output) => {
            eprintln!("protoc version: {}", String::from_utf8_lossy(&output.stdout));
        }
        Err(e) => {
            eprintln!("警告: protoc 不可用: {}", e);
        }
    }

    // 检查 out_dir 是否可创建
    let out_dir = Path::new("src/proto");
    eprintln!("out_dir: {}", out_dir.display());
    eprintln!("out_dir exists: {}", out_dir.exists());
    if !out_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(out_dir) {
            eprintln!("警告: 无法创建 out_dir: {}", e);
        }
    }

    // 编译 Protocol Buffers 定义
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/proto")
        .compile(&[proto_file.to_str().unwrap()], &[proto_dir.to_str().unwrap()])?;
    Ok(())
}
