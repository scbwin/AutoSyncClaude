use std::fs;
use std::io::Result;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    // 获取项目根目录
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not set");

    // 构建 proto 文件路径（相对于项目根目录）
    let proto_file = format!("{}/../proto/sync.proto", manifest_dir);
    let proto_include = format!("{}/../proto", manifest_dir);

    // 获取输出目录
    let out_dir = std::env::var("OUT_DIR").unwrap();

    // 打印调试信息
    println!("cargo:warning=proto_file: {}", proto_file);
    println!("cargo:warning=proto_include: {}", proto_include);
    println!("cargo:warning=out_dir: {}", out_dir);

    // 配置 tonic_build
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(&out_dir)
        .compile_well_known_types(true)
        .compile(&[&proto_file], &[&proto_include])
        .map_err(|e| {
            println!("cargo:warning=tonic_build error: {:?}", e);
            std::io::Error::new(std::io::ErrorKind::Other, e)
        })?;

    // tonic_build 生成的文件名基于 proto 文件名，但可能有所不同
    // 让我们列出所有生成的 .rs 文件
    if let Ok(entries) = fs::read_dir(&out_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "rs").unwrap_or(false) {
                println!("cargo:warning=Generated file: {:?}", path.file_name());
            }
        }
    }

    // 检查 sync.rs 是否存在，如果不存在，查找第一个 .rs 文件并复制
    let sync_rs = Path::new(&out_dir).join("sync.rs");
    if !sync_rs.exists() {
        // 查找任何 .rs 文件
        if let Ok(entries) = fs::read_dir(&out_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "rs").unwrap_or(false) {
                    // 复制第一个找到的 .rs 文件为 sync.rs
                    if let Ok(content) = fs::read_to_string(&path) {
                        fs::write(&sync_rs, content)?;
                        println!("cargo:warning=Copied {:?} to sync.rs", path.file_name());
                        break;
                    }
                }
            }
        }
    }

    // 如果还是不存在，创建一个空的占位符并打印错误
    if !sync_rs.exists() {
        println!("cargo:warning=ERROR: sync.rs not found in OUT_DIR!");
        println!("cargo:warning=Creating minimal placeholder to allow compilation to fail clearly");
        fs::write(
            &sync_rs,
            "// Proto file not generated - check build.rs configuration\n",
        )?;
    }

    // 复制到 src/proto 供 IDE 使用
    let src_proto_dir = Path::new(&manifest_dir).join("src/proto");
    fs::create_dir_all(&src_proto_dir)?;
    if sync_rs.exists() {
        let _ = fs::copy(&sync_rs, src_proto_dir.join("sync.rs"));
    }

    // 重新运行此构建脚本如果 proto 文件发生变化
    println!("cargo:rerun-if-changed=../proto/sync.proto");

    Ok(())
}
