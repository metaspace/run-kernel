// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    command::{self, CheckExitCode, Command},
    config::{self, RunConfig, Serial},
};
use anyhow::{anyhow, Context};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::Stdio,
    thread::sleep,
    time::Duration,
};

type Result<T = ()> = anyhow::Result<T>;
const INITRD: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/initrd.img"));

fn build_nixos_expression(config: &RunConfig) -> Result<PathBuf> {
    let flake = format!(
        "{}#nixosConfigurations.vm.config.system.build.toplevel",
        config.flake
    );
    Command::new("nix")
        .arg("build")
        .arg(flake)
        .spawn()?
        .wait()?
        .check_status()?;
    Ok(std::fs::read_link("result")?)
}

pub(crate) fn run_kernel(config: &RunConfig) -> Result {
    let build_dir = build_nixos_expression(config)?;

    let virtio_daemons = config
        .shares_iter()?
        .into_iter()
        .map(|share| {
            let (tag, path) = share;
            virtiofsd_cmd(config, tag.as_str(), &path)?
                .spawn()
                .context("Failed to spawn virtiofsd")
        })
        .collect::<Result<Vec<_>>>()?;

    let virtio_guard = drop_guard::guard(virtio_daemons, |daemons| {
        for mut daemon in daemons {
            daemon.kill().expect("Failed to kill daemon");
        }
    });

    sleep(Duration::from_secs(1));

    let mut initrd = tempfile::NamedTempFile::new()?;
    initrd.write_all(INITRD)?;
    let initrd_path = initrd.into_temp_path();

    let mut qemu_guard = drop_guard::guard(
        qemu_cmd(config, &build_dir, &initrd_path)?
            .spawn()
            .context("Failed to start qemu")?,
        |mut qemu| {
            let _ = qemu.kill();
        },
    );

    if config.serial == Serial::StdIO {
        qemu_guard.wait()?;
        return Ok(());
    }

    if config.ping_ssh || config.ssh {
        if let Err(e) = ping_vm_ssh(config) {
            return Err(e);
        }
    }

    if config.ssh {
        let mut command = vm_ssh_cmd(config);
        if let Some(ssh_command) = &config.ssh_command {
            command.arg(ssh_command);
        }
        command.spawn()?.wait()?;
        vm_ssh_shutdown(config).context("Failed to shut down VM via ssh")?;
    } else {
        qemu_guard.wait()?;
        return Ok(());
    }

    {
        let mut ok = false;
        for _ in 0..10 {
            if qemu_guard.try_wait()?.is_some() {
                ok = true;
                break;
            }
            sleep(Duration::from_millis(500));
        }
        if !ok {
            print!("Qemu did not shut down, killing it");
            drop(qemu_guard);
        }
    }

    println!("qemu terminated");

    drop(virtio_guard);

    Ok(())
}

fn virtiofs_get_socket_path(config: &RunConfig, path: &Path) -> Result<PathBuf> {
    let mut socket_path = PathBuf::from(&config.virtiofsd_socket_dir);
    let path = std::path::absolute(std::fs::canonicalize(path)?)?;
    socket_path.push(path.as_os_str().to_string_lossy().replace('/', "_"));
    Ok(socket_path)
}

fn virtiofsd_cmd(config: &RunConfig, tag: &str, path: &Path) -> Result<Command> {
    let socket_path = virtiofs_get_socket_path(config, path)?;
    let mut command = Command::new("podman");
    command
        .args(["unshare", "--", &config.virtiofsd])
        .arg("--socket-path")
        .arg(socket_path)
        .arg("--tag")
        .arg(tag)
        .arg("--shared-dir")
        .arg(path)
        .arg("--sandbox=none");

    Ok(command)
}

fn vm_ssh_cmd(_config: &RunConfig) -> Command {
    let mut command = Command::new("ssh");

    command.args([
        "-l",
        "root",
        "-p",
        "10022",
        "-o",
        "UserKnownHostsFile=/dev/null",
        "-o",
        "StrictHostKeyChecking=off",
        "localhost",
    ]);

    command
}

fn ping_vm_ssh(config: &RunConfig) -> Result<()> {
    let mut ping_ok = false;
    for _ in 0..10 {
        let mut command = vm_ssh_cmd(config);
        command.arg("true");
        let exit_code = command
            .spawn()?
            .wait()?
            .code()
            .ok_or(anyhow!("Failed to get exit status of ssh ping process"))?;
        if exit_code == 0 {
            ping_ok = true;
            break;
        }
        sleep(Duration::from_millis(300));
    }

    if !ping_ok {
        return Err(anyhow!("Failed to ping VM"));
    }

    Ok(())
}

fn vm_ssh_shutdown(config: &RunConfig) -> Result<()> {
    let mut command = vm_ssh_cmd(config);
    command.arg("poweroff");
    // ssh will return nonzero exit status when connection is dropped
    command.spawn()?.wait()?;
    Ok(())
}

fn virtiofs_qemu_args(
    config: &RunConfig,
    tag: &str,
    path: &Path,
) -> Result<impl IntoIterator<Item = impl AsRef<str>>> {
    let socket_path = virtiofs_get_socket_path(config, path)?;

    Ok(vec![
        format!("-chardev"),
        format!(
            "socket,id=virtiofs-{tag},path={}",
            socket_path.to_string_lossy()
        ),
        format!("-device"),
        format!("vhost-user-fs-pci,queue-size=1024,chardev=virtiofs-{tag},tag={tag}"),
    ])
}

fn qemu_args<'a>(
    command: &'a mut Command,
    config: &RunConfig,
    build_dir: &Path,
    initrd_path: impl AsRef<Path>,
) -> Result<&'a mut Command> {
    command::qemu_base_args(command, config).args([
        "-name",
        "kernel-test,debug-threads=on",
        "-nic",
        "user,model=virtio-net-pci,hostfwd=tcp:127.0.0.1:10022-:22",
        "-smp",
        &format!("{}", config.smp),
    ]);

    if config.debug {
        command.args(["-s", "-S"]);
    }

    match config.serial {
        Serial::Telnet => command.args(["-serial", "telnet:localhost:4000,server"]),
        Serial::Log => command.args([
            "-chardev",
            "stdio,id=char0,mux=on,logfile=console.log",
            "-mon",
            "chardev=char0",
            "-serial",
            "chardev:char0",
        ]),
        Serial::Disconnected | Serial::StdIO => command.args(["-serial", "mon:stdio"]),
    };

    for (tag, path) in config.shares_iter()? {
        command.args(
            virtiofs_qemu_args(config, tag.as_str(), &path)?
                .into_iter()
                .map(|s| s.as_ref().to_owned()),
        );
    }

    command
        .arg("-object")
        .arg(format!(
            "memory-backend-file,id=mem,size={}G,mem-path=/dev/shm,share=on",
            config.memory_gib
        ))
        .args(["-numa", "node,memdev=mem"]);

    command.arg("-initrd").arg(initrd_path.as_ref());

    //let mut root_port = 3;
    // TODO: drives

    match config.boot {
        config::Boot::Direct => {
            command.args(["-kernel", &config.kernel, "-append"]);

            let console = if cfg!(target_arch = "aarch64") {
                "ttyAMA0"
            } else {
                "ttyS0"
            };
            let mut kernel_args = format!(
                "console={console} nokaslr rdinit=/init init={}/init",
                build_dir.to_string_lossy()
            );
            if let Some(args) = &config.kernel_extra_args {
                kernel_args += " ";
                kernel_args += args;
            }
            command.arg(kernel_args);
        }
    }

    // TODO: Remote

    command.args(&config.qemu_extra_args);
    Ok(command)
}

fn qemu_cmd(
    config: &RunConfig,
    build_dir: &Path,
    initrd_path: impl AsRef<Path>,
) -> Result<Command> {
    let mut command = Command::new(&config.qemu);
    qemu_args(&mut command, config, build_dir, initrd_path)?;
    if config.serial != Serial::StdIO {
        command.stdin(Stdio::null());
    }
    Ok(command)
}
