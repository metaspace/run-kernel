use anyhow::{anyhow, Context};
use std::path::{Path, PathBuf};

type Result<T = ()> = anyhow::Result<T>;

trait ExitOk {
    fn success(self) -> Result;
}

impl ExitOk for std::process::ExitStatus {
    fn success(self) -> Result {
        if std::process::ExitStatus::success(&self) {
            Ok(())
        } else {
            Err(anyhow!("Process failed: {}", self.code().unwrap()))
        }
    }
}

fn main() -> Result {
    std::process::Command::new("cargo")
        .current_dir("init")
        .arg("build")
        .arg("--release")
        .spawn()?
        .wait()?
        .success()?;

    let mut init_path = PathBuf::from("./init");
    init_path.push("target");
    init_path.push("x86_64-unknown-linux-musl");
    init_path.push("release");
    init_path.push("init");

    let content = std::iter::once((
        cpio::NewcBuilder::new("init")
            .uid(0)
            .gid(0)
            .mode(0o550)
            .ino(1)
            .set_mode_file_type(cpio::newc::ModeFileType::Regular),
        std::fs::File::open(init_path).context("Failed to open {init_path}")?,
    ));

    let out_dir = std::env::var_os("OUT_DIR").ok_or(anyhow!("Could not read `OUT_DIR`"))?;
    let out_dir = Path::new(&out_dir);
    std::fs::create_dir_all(&out_dir).context(format!(
        "Could not create `OUT_DIR`: {}",
        out_dir.to_string_lossy()
    ))?;

    let target_path = out_dir.join("initrd.img");
    let target_file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(&target_path)
        .context(format!(
            "Could not open output file: {}",
            target_path.to_string_lossy()
        ))?;

    let mut target_file = zstd::stream::Encoder::new(target_file, 10)?;

    cpio::write_cpio(content, &mut target_file)?;

    target_file.finish()?;

    Ok(())
}
