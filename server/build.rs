use std::io::Result;

fn main() -> Result<()> {
    // 编译 Protocol Buffers 定义
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/proto")
        .compile(&["../proto/sync.proto"], &["../proto"])?;
    Ok(())
}
