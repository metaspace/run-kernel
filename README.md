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
run-kernel --ssh=true --share=share:./
```

# Requirements

 - cargo
 - virtiofsd
 - cpio
 - qemu
 - podman (for `unshare` subcommand)

# License

[GPL 3.0](COPYING)

