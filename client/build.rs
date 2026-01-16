use std::fs;
use std::io::Result;
use std::path::{Path, PathBuf};

// 手动规范化路径，避免产生 Windows UNC 路径 (\\?\)
fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        use std::path::Component;
        match component {
            Component::Prefix(p) => result.push(p.as_os_str()),
            Component::RootDir => result.push(component.as_os_str()),
            Component::Normal(s) => result.push(s),
            Component::CurDir => {}
            Component::ParentDir => {
                if !result.pop() {
                    result.push("..");
                }
            }
        }
    }
    result
}

fn main() -> Result<()> {
    // 获取项目根目录（CARGO_MANIFEST_DIR 指向 client 目录）
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR 环境变量未设置");

    // proto 文件位于 client 的父目录，手动规范化路径避免 UNC 格式
    let proto_path = Path::new(&manifest_dir).join("../proto");
    let proto_dir = normalize_path(&proto_path);
    let proto_file = proto_dir.join("sync.proto");

    // 使用 OUT_DIR 作为输出目录（Cargo 构建脚本的标准做法）
    let out_dir_str = std::env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir_str);

    // 编译 Protocol Buffers 定义到 OUT_DIR
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(&out_dir)
        .compile(
            &[proto_file.to_str().unwrap()],
            &[proto_dir.to_str().unwrap()],
        )?;

    // 查找实际生成的 .rs 文件
    let generated_files = fs::read_dir(&out_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "rs").unwrap_or(false))
        .collect::<Vec<_>>();

    // 创建一个固定名称的文件，包含实际生成的文件内容
    let sync_wrapper = out_dir.join("sync.rs");
    if let Some(first_rs) = generated_files.first() {
        let content = fs::read_to_string(first_rs)?;
        fs::write(&sync_wrapper, content)?;
        println!("cargo:warning=Generated proto wrapper from: {:?}", first_rs);
    }

    // 同时也复制到 src/proto 用于 IDE 支持
    let src_proto_dir = Path::new(&manifest_dir).join("src/proto");
    if !src_proto_dir.exists() {
        let _ = fs::create_dir_all(&src_proto_dir);
    }
    if sync_wrapper.exists() {
        let _ = fs::copy(&sync_wrapper, src_proto_dir.join("sync.rs"));
    }

    // 重新运行此构建脚本如果 proto 文件发生变化
    println!("cargo:rerun-if-changed=../proto/sync.proto");

    Ok(())
}
