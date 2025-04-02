// SPDX-License-Identifier: GPL-3.0-only

use std::iter;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use config_manager::config;
use config_manager::ConfigInit;
use config_manager::Flatten;
use serde::Deserialize;

#[derive(Debug)]
#[config(file(
    format = "toml",
    default = "./run-kernel.cfg",
    clap(long = "config-file")
))]
pub(crate) struct Args {
    #[source(clap(long = "print-config"), default)]
    pub(crate) print_config: bool,

    #[flatten]
    pub(crate) config: Config,
}

#[derive(Deserialize, Flatten, Debug)]
pub(crate) struct Config {
    #[flatten]
    pub(crate) run_config: RunConfig,
}

#[derive(Deserialize, Default, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Serial {
    #[default]
    Disconnected,
    StdIO,
    Telnet,
    Log,
}

#[derive(Deserialize, Flatten, Debug)]
#[table = "run"]
pub(crate) struct RunConfig {
    #[source(clap, config, default)]
    pub(crate) debug: bool,

    #[source(clap, config, default = 2)]
    pub(crate) smp: u32,

    #[source(clap, config, default = "String::from(\"linux-build/vmlinux\")")]
    pub(crate) kernel: String,

    #[source(clap, config, default)]
    pub(crate) share: Vec<String>,

    #[source(clap, config, default)]
    pub(crate) serial: Serial,

    #[source(clap, config, default)]
    pub(crate) ssh: bool,

    #[source(clap, config, default = "None")]
    pub(crate) ssh_command: Option<String>,

    #[source(clap, config, default)]
    pub(crate) ping_ssh: bool,

    #[source(clap, config, default)]
    pub(crate) boot: Boot,

    #[source(
        clap,
        config,
        default = "format!(\"qemu-system-{}\", std::env::consts::ARCH)"
    )]
    pub(crate) qemu: String,

    #[source(clap, config, default = 4)]
    pub(crate) memory_gib: u32,

    #[source(clap, config, default = "None")]
    pub(crate) kernel_extra_args: Option<String>,

    #[source(clap, config, default)]
    pub(crate) qemu_extra_args: Vec<String>,

    #[source(clap, config, default = "String::from(\"/usr/lib/virtiofsd\")")]
    pub(crate) virtiofsd: String,

    #[source(clap, config, default = "String::from(\"/tmp/\")")]
    pub(crate) virtiofsd_socket_dir: String,

    #[source(clap, config, default = "String::from(\"./nixos\")")]
    pub(crate) flake: String,
}

fn validate_share(share: impl AsRef<str>) -> Result<(String, PathBuf)> {
    let expr = regex::Regex::new(r"(^[[:alpha:]]+):(.+)$")?;
    let capture = expr
        .captures(share.as_ref())
        .ok_or(anyhow!("Invalid share path: {}", share.as_ref()))?;
    let tag = capture.get(1).unwrap();
    let path = capture.get(2).unwrap();
    Ok((
        tag.as_str().to_owned(),
        PathBuf::from(path.as_str().to_owned()),
    ))
}

impl RunConfig {
    pub fn shares_iter(&self) -> Result<impl IntoIterator<Item = (String, PathBuf)>> {
        Ok(iter::once("store:/nix/store")
            .chain(self.share.iter().map(|s| s.as_ref()))
            .map(validate_share)
            .collect::<Result<Vec<_>>>()?)
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub(crate) enum Boot {
    Direct,
}

impl Default for Boot {
    fn default() -> Self {
        Self::Direct
    }
}

pub(crate) fn get_config() -> Result<Args> {
    let args = Args::parse().context("Failed to parse config")?;

    Ok(args)
}
