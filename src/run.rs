// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    command::{self, Command},
    config::{self, RunConfig, Serial},
};
use anyhow::{anyhow, Context, Result};
use std::{process::Stdio, thread::sleep, time::Duration};

pub(crate) fn run_kernel(config: &RunConfig) -> Result<()> {
    let virtiofsd = if config.share {
        let x = Some(virtiofsd_cmd(config)?.spawn()?);
        sleep(Duration::from_secs(1));
        x
    } else {
        None
    };

    let mut qemu = qemu_cmd(config)?.spawn().context("Failed to start qemu")?;

    if config.serial == Serial::StdIO {
        qemu.wait()?;
        return Ok(());
    }

    // TODO: kill with drop guard
    if config.ping_ssh || config.ssh {
        if let Err(e) = ping_vm_ssh(config) {
            qemu.kill()?;
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
        qemu.wait()?;
        return Ok(());
    }

    {
        let mut ok = false;
        for _ in 0..10 {
            if qemu.try_wait()?.is_some() {
                ok = true;
                break;
            }
            sleep(Duration::from_millis(500));
        }
        if !ok {
            print!("Qemu did not shut down, killing it");
            qemu.kill()?;
        }
    }

    println!("qemu terminated");

    // TODO: kill with drop guard
    if let Some(mut virtiofsd) = virtiofsd {
        virtiofsd.kill()?;
    }

    Ok(())
}

fn virtiofsd_cmd(config: &RunConfig) -> Result<Command> {
    use std::env;

    let cwd = env::current_dir()?;

    let mut command = Command::new("podman");
    command
        .args(["unshare", "--", &config.virtiofsd])
        .arg("--socket-path")
        .arg(&config.virtiofsd_socket)
        .arg("--shared-dir")
        .arg(cwd)
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

fn qemu_args<'a>(command: &'a mut Command, config: &RunConfig) -> &'a mut Command {
    command::qemu_base_args(command, config).args([
        "-name",
        "kernel-test,debug-threads=on",
        "-nic",
        "user,model=virtio-net-pci,hostfwd=tcp:127.0.0.1:10022-:22",
        "-smp",
    ]);
    command.arg(format!("{}", config.smp));

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

    if config.share {
        command
            .arg("-chardev")
            .arg(format!(
                "socket,id=virtiofs0,path={}",
                config.virtiofsd_socket
            ))
            .args([
                "-device",
                "vhost-user-fs-pci,queue-size=1024,chardev=virtiofs0,tag=sources",
            ])
            .arg("-object")
            .arg(format!(
                "memory-backend-file,id=mem,size={}G,mem-path=/dev/shm,share=on",
                config.memory_gib
            ))
            .args(["-numa", "node,memdev=mem"]);
    }

    //let mut root_port = 3;
    // TODO: drives

    command
        .arg("-drive")
        .arg(format!("file={},if=virtio,format=qcow2", config.image));

    match config.boot {
        config::Boot::Direct => {
            command.args(["-kernel", &config.kernel, "-append"]);

            let console = if cfg!(target_arch = "aarch64") {
                "ttyAMA0"
            } else {
                "ttyS0"
            };
            let mut kernel_args =
                format!("console={console} nokaslr rdinit=/sbin/init root=/dev/vda1");
            if let Some(args) = &config.kernel_extra_args {
                kernel_args.push(' ');
                kernel_args.push_str(args);
            }
            command.arg(kernel_args);
        }
        config::Boot::Native => {}
    }

    // TODO: Remote

    command.args(&config.qemu_extra_args);
    command
}

fn qemu_cmd(config: &RunConfig) -> Result<Command> {
    let mut command = Command::new(&config.qemu);
    qemu_args(&mut command, config);
    if config.serial != Serial::StdIO {
        command.stdin(Stdio::null());
    }
    Ok(command)
}
