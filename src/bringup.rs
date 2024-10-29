// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    command::{self, CheckExitCode, Command},
    config::{BringupConfig, Config},
};
use anyhow::{Context, Result};
use mktemp::Temp;
use std::{
    fs::File,
    io::{copy, Read, Write as _},
    path::{Path, PathBuf},
    process::Stdio,
};

struct ProgressRead<R: Read> {
    inner: R,
    bar: indicatif::ProgressBar,
}

impl<R: Read> Read for ProgressRead<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let Self { inner, bar } = self;
        let n = inner.read(buf)?;
        bar.inc(n as u64);
        Ok(n)
    }
}

fn get_seed_image(config: &Config) -> Result<()> {
    let mut out = match File::create_new(&config.bringup_config.seed_image_path) {
        Ok(out) => out,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                println!("Skipping seed image download: file exists");
                return Ok(());
            } else {
                return Err(e).context("Failed to create seed image file");
            }
        }
    };
    println!(
        "Downloading seed image {} to {}",
        &config.bringup_config.seed_image_url, &config.bringup_config.seed_image_path
    );
    let resp = reqwest::blocking::get(&config.bringup_config.seed_image_url)?;
    let bar = match resp.content_length() {
        Some(length) => indicatif::ProgressBar::new(length),
        None => indicatif::ProgressBar::new_spinner(),
    };
    let _: u64 =
        copy(&mut ProgressRead { inner: resp, bar }, &mut out).context("failed to copy content")?;

    // TODO: Download checksum and verify image
    Ok(())
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
    qemu_img_cmd(
        &config.run_config.image,
        config.bringup_config.image_size,
        Some(&config.bringup_config.seed_image_path),
    )?
    .spawn()?
    .wait()?
    .check_status()?;

    #[cfg(target_arch = "aarch64")]
    qemu_img_cmd(
        &config.run_config.qemu_varstore_image_path,
        byte_unit::Byte::from_u64_with_unit(64, byte_unit::Unit::MiB).unwrap(),
        None::<&str>,
    )?
    .spawn()?
    .wait()?
    .check_status()?;

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

fn qemu_img_cmd(
    path: impl AsRef<str>,
    size: byte_unit::Byte,
    backing_path: Option<impl AsRef<str>>,
) -> Result<Command> {
    let mut command = Command::new("qemu-img");

    command.args(["create", "-f", "qcow2"]);
    if let Some(backing_path) = backing_path {
        let backing_path = PathBuf::from(backing_path.as_ref()).canonicalize()?;
        command.args(["-b", &backing_path.to_string_lossy(), "-F", "qcow2"]);
    }

    command.args([
        path.as_ref(),
        &format!(
            "{}M",
            size.get_adjusted_unit(byte_unit::Unit::MiB).get_value()
        ),
    ]);

    Ok(command)
}

fn qemu_init_command(config: &Config, meta_image_path: &Path) -> Command {
    let mut command = Command::new(&config.run_config.qemu);
    command::qemu_base_args(&mut command, &config.run_config);

    #[cfg(target_arch = "aarch64")]
    {
        command.args([
            "-drive",
            &format!(
                "if=pflash,format=raw,file={file}",
                file = config.run_config.qemu_efi_image_path
            ),
        ]);
        command.args([
            "-drive",
            &format!(
                "if=pflash,file={file}",
                file = config.run_config.qemu_varstore_image_path
            ),
        ]);
    }

    command
        .args(["-smp", "2"])
        .args(["-serial", "mon:stdio", "-nic", "user,model=virtio-net-pci"])
        .arg("-drive")
        .arg(format!(
            "id=boot,file={path},format=qcow2,if=virtio,media=disk,read-only=no",
            path = &config.run_config.image
        ))
        .arg("-drive")
        .args([format!(
            "id=seed,file={path},format=raw,if=virtio,media=disk,read-only=yes",
            path = meta_image_path.to_string_lossy()
        )])
        .stdin(Stdio::null());
    command
}
