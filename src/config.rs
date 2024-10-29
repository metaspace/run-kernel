// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Context;
use anyhow::Result;
use config_manager::config;
use config_manager::ConfigInit;
use config_manager::Flatten;
use serde::Deserialize;
use serde::Serialize;

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
    #[source(clap, default)]
    pub(crate) bringup: bool,

    #[flatten]
    pub(crate) run_config: RunConfig,

    #[flatten]
    pub(crate) bringup_config: BringupConfig,
}

#[derive(Deserialize, Default, Debug, PartialEq)]
#[serde(rename_all="lowercase")]
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
    pub(crate) share: bool,

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

    #[source(clap, config, default = "format!(\"qemu-system-{}\", std::env::consts::ARCH)")]
    pub(crate) qemu: String,

    #[source(clap, config, default = 4)]
    pub(crate) memory_gib: u32,

    #[source(clap, config, default = "None")]
    pub(crate) kernel_extra_args: Option<String>,

    #[source(clap, config, default)]
    pub(crate) qemu_extra_args: Vec<String>,

    #[source(clap, config, default = "String::from(\"vm-image/vm.qcow2\")")]
    pub(crate) image: String,

    #[source(clap, config, default = "String::from(\"/usr/lib/virtiofsd\")")]
    pub(crate) virtiofsd: String,

    #[source(clap, config, default = "String::from(\"/tmp/vhostqemu\")")]
    pub(crate) virtiofsd_socket: String,
}

#[derive(Serialize, Deserialize, Flatten, Clone, Debug)]
#[table = "bringup"]
pub(crate) struct BringupConfig {
    #[source(clap, config, default)]
    pub(crate) packages: Vec<String>,

    #[source(clap, config, default)]
    pub(crate) commands: Vec<Vec<String>>,

    #[source(
        clap(long = "seed-image-url"),
        config,
        default = "String::from(\"https://cloud.debian.org/images/cloud/bookworm/latest/debian-12-genericcloud-amd64.qcow2\")"
    )]
    pub(crate) seed_image_url: String,

    #[source(
        clap(long = "seed-image-path"),
        config,
        default = "String::from(\"vm-image/seed.img\")"
    )]
    pub(crate) seed_image_path: String,

    #[source(clap(long = "image-size-gb"), config, default = 50)]
    pub(crate) image_size_gb: u32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub(crate) enum Boot {
    Native,
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
