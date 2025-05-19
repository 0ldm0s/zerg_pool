use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 获取项目根目录绝对路径
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    
    let proto_path = manifest_dir.join("proto/task.proto");
    let proto_dir = manifest_dir.join("proto");
    let out_dir = manifest_dir.join("src/proto");

    // 确保输出目录存在
    std::fs::create_dir_all(&out_dir)?;

    // 清理旧文件
    let _ = std::fs::remove_file(out_dir.join("zerg.pool.rs"));

    // 强制设置输出文件名为zergpool.rs
    let mut config = prost_build::Config::new();
    config.out_dir(&out_dir)
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .file_descriptor_set_path(out_dir.join("descriptor.bin"))
        .compile_protos(&[proto_path.clone()], &[proto_dir])?;

    // 重命名输出文件
    std::fs::rename(
        out_dir.join("zerg.pool.rs"),
        out_dir.join("zergpool.rs")
    )?;

    // 创建mod.rs文件
    std::fs::write(
        manifest_dir.join("src/proto/mod.rs"),
        "pub mod zergpool;\n"
    )?;

    // 重新编译当proto文件变化时
    println!("cargo:rerun-if-changed={}", proto_path.display());

    Ok(())
}