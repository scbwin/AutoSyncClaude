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

    // 确保 out_dir 存在
    let out_dir = Path::new("src/proto");
    if !out_dir.exists() {
        let _ = std::fs::create_dir_all(out_dir);
    }

    // 生成 proto/mod.rs 文件
    let mod_rs_path = out_dir.join("mod.rs");
    let mod_content = r#"// 此文件由 build.rs 自动生成
// 请勿手动编辑

// 引入 tonic-build 生成的 protobuf 代码
// sync.proto 会生成 sync.rs，其中包含 claude_sync 模块
pub mod sync;
"#;
    std::fs::write(&mod_rs_path, mod_content).expect("无法写入 proto/mod.rs");

    // 编译 Protocol Buffers 定义
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/proto")
        .compile(
            &[proto_file.to_str().unwrap()],
            &[proto_dir.to_str().unwrap()],
        )?;
    Ok(())
}
