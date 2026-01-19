fn main() {
    tauri_build::build();

    // 生成 protobuf 代码
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let proto_file = format!("{}/../../proto/sync.proto", manifest_dir);
    let proto_include = format!("{}/../../proto", manifest_dir);

    let proto_dir = std::path::Path::new(&manifest_dir).join("src/proto");
    std::fs::create_dir_all(&proto_dir).unwrap();

    tonic_build::configure()
        .build_client(true)
        .build_server(false)
        .out_dir(&proto_dir)
        .compile_well_known_types(true)
        .compile(&[&proto_file], &[&proto_include])
        .expect("protobuf compile failed");

    println!("cargo:rerun-if-changed=../../proto/sync.proto");
}
