use std::io::Result;

fn main() -> Result<()> {
    // 获取项目根目录（CARGO_MANIFEST_DIR 指向 server 目录）
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR 环境变量未设置");

    // proto 文件位于 server 的父目录
    let proto_dir = format!("{}/../proto", manifest_dir);
    let proto_file = format!("{}/sync.proto", proto_dir);

    // 编译 Protocol Buffers 定义
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/proto")
        .compile(&[&proto_file], &[&proto_dir])?;
    Ok(())
}
