// Proto 生成脚本
// 使用 tonic-build 生成 Rust 代码

use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 项目根目录
    let project_root = Path::new("D:\\dev_syncClaude");

    // Proto 文件路径
    let proto_file = project_root.join("proto/sync.proto");
    let proto_include = project_root.join("proto");

    // 检查 proto 文件是否存在
    if !proto_file.exists() {
        eprintln!("Proto file not found: {:?}", proto_file);
        return Err("Proto file not found".into());
    }

    println!("Generating proto code from: {:?}", proto_file);

    // 为服务器生成代码
    let server_proto_dir = project_root.join("server/src/proto");
    fs::create_dir_all(&server_proto_dir)?;

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(&server_proto_dir)
        .compile_well_known_types(true)
        .compile(&[&proto_file], &[&proto_include])?;

    println!("Server proto files generated to: {:?}", server_proto_dir);

    // 为客户端生成代码
    let client_proto_dir = project_root.join("client/src/proto");
    fs::create_dir_all(&client_proto_dir)?;

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(&client_proto_dir)
        .compile_well_known_types(true)
        .compile(&[&proto_file], &[&proto_include])?;

    println!("Client proto files generated to: {:?}", client_proto_dir);

    Ok(())
}
