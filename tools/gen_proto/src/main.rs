use std::env;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 获取项目根目录（假设从 tools/gen_proto 运行）
    let project_root = Path::new("..").canonicalize()?;
    let proto_file = project_root.join("proto/sync.proto");
    let proto_include = project_root.join("proto");

    println!("Project root: {:?}", project_root);
    println!("Proto file: {:?}", proto_file);

    // 检查 proto 文件
    if !proto_file.exists() {
        eprintln!("Error: Proto file not found: {:?}", proto_file);
        return Err("Proto file not found".into());
    }

    // 为服务器生成代码
    let server_proto_dir = project_root.join("server/src/proto");
    fs::create_dir_all(&server_proto_dir)?;
    println!("Server proto dir: {:?}", server_proto_dir);

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(&server_proto_dir)
        .compile_well_known_types(true)
        .compile(&[&proto_file], &[&proto_include])?;

    println!("Server proto files generated successfully!");

    // 检查生成的文件
    let entries = fs::read_dir(&server_proto_dir)?;
    println!("Generated files in server/src/proto:");
    for entry in entries {
        let entry = entry?;
        println!("  - {:?}", entry.file_name());
    }

    // 为客户端生成代码
    let client_proto_dir = project_root.join("client/src/proto");
    fs::create_dir_all(&client_proto_dir)?;

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(&client_proto_dir)
        .compile_well_known_types(true)
        .compile(&[&proto_file], &[&proto_include])?;

    println!("Client proto files generated successfully!");

    Ok(())
}
