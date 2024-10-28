// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    command::{self, CheckExitCode, Command},
    config::{BringupConfig, Config},
};
use anyhow::{Context as _, Result};
use futures::StreamExt as _;
use mktemp::Temp;
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::io::AsyncWriteExt;

async fn get_seed_image_async(config: &Config) -> Result<()> {
    println!(
        "Downloading seed image {} to {}",
        &config.bringup_config.seed_image_url, &config.bringup_config.seed_image_path
    );
    let mut file = tokio::fs::File::create(&config.bringup_config.seed_image_path)
        .await
        .context("Failed to create seed image file")?;
    let request = reqwest::get(&config.bringup_config.seed_image_url).await?;

    let bar = match request.content_length() {
        Some(length) => indicatif::ProgressBar::new(length),
        None => indicatif::ProgressBar::new_spinner(),
    };

    let mut stream = request.bytes_stream();
    while let Some(data) = stream.next().await {
        let data = data?;
        file.write_all(&data).await?;
        bar.inc(data.len() as u64);
    }
    Ok(())
}

fn get_seed_image(config: &Config) -> Result<()> {
    if std::path::Path::new(&config.bringup_config.seed_image_path).exists() {
        println!("Skipping seed image download: file exists");
        return Ok(());
    }

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    // TODO: Download and check signature
    rt.block_on(get_seed_image_async(config))
}

fn generate_meta(config: &BringupConfig) -> Result<(Temp, PathBuf)> {
    let mut template = tinytemplate::TinyTemplate::new();
    template.add_template(
        "user-data",
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/user-data")),
    )?;

    let rendered = template.render("user-data", config)?;

    let meta_tempdir = Temp::new_dir()?;

    let mut user_data_file_path = PathBuf::from(meta_tempdir.as_path());
    user_data_file_path.push("user-data");
    let mut user_data_file = File::create(&user_data_file_path)?;
    user_data_file.write_all(rendered.as_bytes())?;

    let mut meta_data_file_path = PathBuf::from(meta_tempdir.as_path());
    meta_data_file_path.push("meta-data");
    let mut meta_data_file = File::create(&meta_data_file_path)?;
    meta_data_file.write_all(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/meta-data")).as_bytes(),
    )?;

    let image_tmpdir = Temp::new_dir()?;
    let mut image_path = PathBuf::from(image_tmpdir.as_path());
    image_path.push("init.img");

    let mut cmd = joliet_cmd(
        [user_data_file_path, meta_data_file_path].into_iter(),
        image_path.as_path(),
    )?;

    cmd.spawn()?.wait()?.check_status()?;

    Ok((image_tmpdir, image_path))
}

pub(crate) fn bring_up(config: &Config) -> Result<()> {
    get_seed_image(config)?;
    qemu_img_cmd(config)?.spawn()?.wait()?.check_status()?;
    let (_dir, image_path) = generate_meta(&config.bringup_config)?;
    let mut command = qemu_init_command(config, &image_path);
    command.spawn()?.wait()?.check_status()?;
    Ok(())
}

fn joliet_cmd(
    input_files: impl Iterator<Item = impl AsRef<std::ffi::OsStr>>,
    output_path: &Path,
) -> Result<Command> {
    let mut command = Command::new("mkisofs");

    command
        .arg("-output")
        .arg(output_path)
        .args(["-volid", "cidata", "-joliet", "-rock"])
        .args(input_files);

    Ok(command)
}

fn qemu_img_cmd(config: &Config) -> Result<Command> {
    let mut command = Command::new("qemu-img");
    let backing_file = std::path::PathBuf::from(&config.bringup_config.seed_image_path);
    let backing_file = backing_file.canonicalize()?;
    command
        .args(["create", "-f", "qcow2"])
        .arg("-b")
        .arg(backing_file)
        .args(["-F", "qcow2"])
        .arg(&config.run_config.image)
        .arg(format!("{}G", config.bringup_config.image_size_gb));

    Ok(command)
}

fn qemu_init_command(config: &Config, meta_image_path: &Path) -> Command {
    let mut command = Command::new(&config.run_config.qemu);
    command::qemu_base_args(&mut command, &config.run_config);
    command
        .arg("-smp")
        .arg("2")
        .args(["-serial", "mon:stdio", "-nic", "user,model=virtio-net-pci"])
        .arg("-drive")
        .arg(format!(
            "id=boot,file={},format=qcow2,if=virtio,media=disk,read-only=no",
            &config.run_config.image
        ))
        .arg("-drive")
        .arg({
            let mut arg = std::ffi::OsString::from("id=seed,file=");
            arg.push(meta_image_path);
            arg.push(",format=raw,if=virtio,media=disk,read-only=yes");
            arg
        })
        .stdin(Stdio::null());
    command
}
