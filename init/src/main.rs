use anyhow::{anyhow, Context};
use std::{os::unix::process::CommandExt, process::Command};
use sys_mount::Unmount;

type Result<T = ()> = anyhow::Result<T>;

fn get_init(cmdline: &[u8]) -> Result<String> {
    std::str::from_utf8(cmdline)?
        .split_whitespace()
        .map(|s| {
            let mut it = s.split('=');
            (it.next().unwrap(), it.next())
        })
        .find(|(key, _value)| *key == "init")
        .ok_or(anyhow!("Could not find `init` in cmdline"))?
        .1
        .ok_or(anyhow!("`init` has no argument"))
        .map(String::from)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_init() {
        let init = "/nix/store/6wdsln96dvnqf428j1j67rwgb6qdvffz-nixos-system-xps-24.11.20241216.3945713/init";
        let cmdline = r"initrd=\EFI\nixos\w5n91dajmx2gkhcsm061didadhildy90-initrd-linux-6.6.66-initrd.efi init=/nix/store/6wdsln96dvnqf428j1j67rwgb6qdvffz-nixos-system-xps-24.11.20241216.3945713/init loglevel=4";
        assert_eq!(init, get_init(cmdline.as_bytes()).unwrap());
    }
}

fn main() -> Result {
    println!("run-kernel init");

    std::fs::create_dir_all("/proc")?;
    let proc = sys_mount::Mount::builder()
        .fstype("proc")
        .flags(sys_mount::MountFlags::RDONLY)
        .mount("none", "/proc")
        .context("Failed to mount proc")?;

    let cmdline = std::fs::read("/proc/cmdline").context("Failed to read cmdline")?;
    let init = get_init(&cmdline).context("Failed to parse cmdline")?;

    // NixOS will skip mounting all the pseudo file systems if it sees /proc mounted.
    proc.unmount(sys_mount::UnmountFlags::empty())?;

    std::fs::create_dir_all("/nix/store")?;
    sys_mount::Mount::builder()
        .fstype("virtiofs")
        .flags(sys_mount::MountFlags::RDONLY)
        .mount("store", "/nix/store")
        .context("Failed to mount virtiofs root")?;

    let error = Command::new(init).exec();
    Err(error).context("Failed to exec init: `{init}`")
}
