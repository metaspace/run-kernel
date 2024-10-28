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

pub(crate) fn qemu_base_args(config: &RunConfig) -> Vec<String> {
    let args = ["-nographic", "-cpu", "host"].into_iter();
    #[cfg(target_os = "linux")]
    let args = args.chain(["-accel", "kvm"].into_iter());
    #[cfg(target_os = "macos")]
    let args = args.chain(["-accel", "hvf"].into_iter());

    #[cfg(target_arch = "aarch64")]
    let args = args.chain(["-machine", "virt"].into_iter());
    #[cfg(target_arch = "x86_64")]
    let args = args.chain(["-machine", "q35"].into_iter());

    args.chain(std::iter::once("-m"))
        .map(ToOwned::to_owned)
        .chain(std::iter::once(format!("{}G", config.memory_gib)))
        .collect()
}
