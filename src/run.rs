// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    command::{self, CheckExitCode, Command},
    config::{self, RunConfig},
};
use anyhow::{anyhow, Context, Result};
use shell_words::split;
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

    if config.stdin {
        qemu.wait()?;
        return Ok(());
    }

    // TODO: kill with drop guard
    if let Err(e) = ping_vm_ssh(config) {
        qemu.kill()?;
        return Err(e);
    }

    if config.ssh {
        let mut command = vm_ssh_cmd(config)?;
        command.spawn()?.wait()?;
        vm_ssh_shutdown(config).context("Failed to shut down VM via ssh")?;
    }

    {
        let mut ok = false;
        for _ in 0..10 {
            if let Some(_) = qemu.try_wait()? {
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

    let mut command = Command::new("podman");
    command.args(vec![
        "unshare",
        "--",
        &config.virtiofsd,
        format!("--socket-path={}", config.virtiofsd_socket).as_str(),
        format!("--shared-dir={}", env::current_dir()?.to_string_lossy()).as_str(),
        "--sandbox=none",
    ]);

    Ok(command)
}

fn vm_ssh_cmd(_config: &RunConfig) -> Result<Command> {
    let mut command = Command::new("ssh");

    command.args(vec![
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

    Ok(command)
}

fn ping_vm_ssh(config: &RunConfig) -> Result<()> {
    let mut ping_ok = false;
    for _ in 0..10 {
        let mut command = vm_ssh_cmd(&config)?;
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
    let mut command = vm_ssh_cmd(config)?;
    command.arg("poweroff");
    // ssh will return nonzero exit status when connection is dropped
    command.spawn()?.wait()?;
    Ok(())
}

fn qemu_args(config: &RunConfig) -> Result<Vec<String>> {
    let mut args = command::qemu_base_args()?;
    args.append(&mut split(
        "-name \
        kernel-test,debug-threads=on \
        -nic user,model=virtio-net-pci,hostfwd=tcp:127.0.0.1:10022-:22",
    )?);

    args.append(&mut split(&format!("-smp {}", config.smp))?);

    if config.debug {
        args.append(&mut split("-s -S")?);
    }

    if config.log {
        args.append(&mut split(
            "-chardev stdio,id=char0,mux=on,logfile=console.log \
             -mon chardev=char0 \
             -serial chardev:char0",
        )?);
    } else {
        args.append(&mut split("-serial mon:stdio")?);
    }

    if config.share {
        args.append(&mut split(&format!(
            "-chardev socket,id=virtiofs0,path='{}' \
             -device vhost-user-fs-pci,queue-size=1024,chardev=virtiofs0,tag=sources \
             -object memory-backend-file,id=mem,size=4G,mem-path=/dev/shm,share=on \
             -numa node,memdev=mem",
            config.virtiofsd_socket
        ))?);
    }

    //let mut root_port = 3;
    // TODO: drives

    args.append(&mut split(&format!(
        "-drive file='{}',if=virtio,format=qcow2",
        config.image,
    ))?);

    match config.boot {
        config::Boot::Direct => {
            args.push("-kernel".into());
            args.push(config.kernel.clone());
            args.push("-append".into());
            args.push("console=ttyS0 nokaslr rdinit=/sbin/init root=/dev/vda1".into());
        }
        config::Boot::Native => {}
    }

    // TODO: Remote

    args.append(&mut config.qemu_args_extra.clone());

    Ok(args.into())
}

fn qemu_cmd(config: &RunConfig) -> Result<Command> {
    let mut command = Command::new(&config.qemu);
    command.args(qemu_args(config)?);
    if !config.stdin {
        command.stdin(Stdio::null());
    }
    Ok(command)
}
