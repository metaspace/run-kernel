// SPDX-License-Identifier: GPL-3.0-only

use anyhow::{anyhow, Result};
use std::process;

use crate::config::RunConfig;

pub(crate) struct Command {
    command: process::Command,
}

impl Command {
    pub(crate) fn new(cmd: &str) -> Self {
        Self {
            command: process::Command::new(cmd),
        }
    }

    pub(crate) fn spawn(&mut self) -> std::io::Result<process::Child> {
        println!("Running command: {:?}", &self.command);
        self.command.spawn()
    }
}

pub(crate) trait CheckExitCode {
    fn check_status(&self) -> Result<()>;
}

impl CheckExitCode for process::ExitStatus {
    fn check_status(&self) -> Result<()> {
        if self.success() {
            Ok(())
        } else {
            Err(anyhow!("Process failed"))
        }
    }
}

impl std::ops::Deref for Command {
    type Target = process::Command;

    fn deref(&self) -> &Self::Target {
        &self.command
    }
}

impl std::ops::DerefMut for Command {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.command
    }
}

pub(crate) fn qemu_base_args<'a>(command: &'a mut Command, config: &RunConfig) -> &'a mut Command {
    {
        command.args(["-nographic", "-cpu", "host"]);
        command.arg("-m");
        command.arg(format!("{}G", config.memory_gib));
        if cfg!(target_os = "linux") {
            command.args(["-accel", "kvm"]);
        }
        if cfg!(target_os = "macos") {
            command.args(["-accel", "hvf"]);
        }
        if cfg!(target_arch = "aarch64") {
            command.args(["-machine", "virt"]);
        }
    }
    command
}
