# `run-kernel` - a tool for running qemu

`run-kernel` is a tool that runs `qemu` for you. It is useful when hacking on
the Linux kernel.

# Install

```shell
cargo install --git https://github.com/metaspace/run-kernel
```

# Configuration

Configuration is collected from (in?)sane defaults, a configuration file and
command line arguments. See [run-kernel.cfg](run-kernel.cfg) for a sample
configuration file with default values.

# Example Usage

Set up a VM image and start a VM, attach to VM with ssh and share current
directory with VM via `virtiofs`:

```shell
mkdir vm-image
run-kernel --print-config=true
run-kernel --bringup=true
run-kernel --ssh=true --share=true
```

## macOS Specific Details

`qemu` on arm64 needs EFI firmware to boot upstream Debian images. You have to
provide this firmware to `run-kernel` so that `qemu` can find it. Pass
`--qemu-efi-image-path=<path>` to `run-kernel` to supply this information.

# Requirements

 - cargo
 - virtiofsd
 - qemu
 - qemu-img
 - mkisofs (`brew install dvdrtools` on macOS)
 - podman (for `unshare` subcommand)
 - uidmap

# License

[GPL 3.0](COPYING)

