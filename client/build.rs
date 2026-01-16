use std::io::Result;
use std::path::Path;

fn main() -> Result<()> {
    // 获取项目根目录（CARGO_MANIFEST_DIR 指向 client 目录）
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR 环境变量未设置");

    // proto 文件位于 client 的父目录
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

    // 编译 Protocol Buffers 定义
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/proto")
        .compile(&[proto_file.to_str().unwrap()], &[proto_dir.to_str().unwrap()])?;
    Ok(())
}
