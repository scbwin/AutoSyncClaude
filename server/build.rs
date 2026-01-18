use std::fs;
use std::io::Result;
use std::path::Path;

fn main() -> Result<()> {
    // 获取项目根目录
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");

    // 构建 proto 文件路径（相对于项目根目录）
    let proto_file = format!("{}/../proto/sync.proto", manifest_dir);
    let proto_include = format!("{}/../proto", manifest_dir);

    // 直接生成到 src/proto 目录，这样可以用标准 mod 声明引用
    let src_proto_dir = Path::new(&manifest_dir).join("src/proto");
    fs::create_dir_all(&src_proto_dir)?;

    // 配置 tonic_build，直接输出到 src/proto/
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(&src_proto_dir)
        .compile_well_known_types(true)
        .compile(&[&proto_file], &[&proto_include])
        .map_err(|e| {
            eprintln!("tonic_build error: {:?}", e);
            std::io::Error::other(e)
        })?;

    // 生成的文件应该是 claude_sync.rs（基于 proto 的 package 名）
    // 如果生成的文件名不同，找出并重命名
    let expected_file = src_proto_dir.join("claude_sync.rs");
    if !expected_file.exists() {
        // 查找生成的 .rs 文件
        if let Ok(entries) = fs::read_dir(&src_proto_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "rs").unwrap_or(false)
                    && path.file_name() != Some(std::ffi::OsStr::new("mod.rs"))
                {
                    // 重命名为 claude_sync.rs
                    if let Ok(content) = fs::read_to_string(&path) {
                        fs::write(&expected_file, content)?;
                        let _ = fs::remove_file(&path);
                        break;
                    }
                }
            }
        }
    }

    // 重新运行此构建脚本如果 proto 文件发生变化
    println!("cargo:rerun-if-changed=../proto/sync.proto");

    Ok(())
}
