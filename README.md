# `run-kernel` - a tool for booting Linux in qemu

`run-kernel` is a tool that boots a Linux kernel into a NixOS userland in
`qemu`. It is useful when hacking on the Linux kernel.

`run-kernel` creates an ephemeral VM instance without any disk images. It does
this by mapping the nix store into the virtual machine with virtiofs. For state
and file transfer, additional directories can be mounted over virtiofs in
read/write mode. To do so, use the `--share` option and modify your NixOS
configuration to mount the file system, or mount it manually. See [this
example](examples/configuration.nix) for details.

# Install

## `cargo`

```shell
cargo install --git https://github.com/metaspace/run-kernel
```

## `nix`

```shell
nix run github:metaspace/run-kernel <args>
```

# Configuration

Configuration is collected from (in?)sane defaults, a configuration file and
command line arguments. See [run-kernel.cfg](run-kernel.cfg) for a sample
configuration file with default values.

**NOTE**: The path of the derivation to boot is currently hard coded to `vm`,
meaning that the following flake path will be built:
`<url>#nixosConfigurations.vm.config.system.build.toplevel`.

# Example Usage

Sart a VM, attach to VM via stdio and share current
directory with VM via `virtiofs`:

```shell
mkdir nixos
cp <path/to/run/kernel/src>/examples/* nixos/
run-kernel --serial=stdio --share=share:./ --flake=./nixos --kernel=<path/to/your/kernel>
```

# Requirements

 - cargo
 - virtiofsd
 - qemu
 - podman (for `unshare` subcommand)
 - nix

## Kernel configuration

The following configuration options can be used to boot NixOS with `run-kernel`.
The set is not minimal, as it includes configuration for virtio-blk and ext4.

Eventually I hope to be able to add configuration validation or augmentation to
`run-kernel`, so that it is easy to update a kernel configuration to work with
`run-kernel`.

```text
# kvm guest
CONFIG_HYPERVISOR_GUEST=y
CONFIG_PARAVIRT=y
CONFIG_KVM_GUEST=y
CONFIG_PVH=y
CONFIG_SERIAL_EARLYCON=y
CONFIG_SERIAL_8250=y
CONFIG_SERIAL_8250_CONSOLE=y

CONFIG_NR_CPUS=8
CONFIG_ACPI=y
CONFIG_ACPI_PROCESSOR=y

# NixOS
CONFIG_TMPFS=y
CONFIG_TMPFS_POSIX_ACL=y
CONFIG_DEVTMPFS=y
CONFIG_CGROUPS=y
CONFIG_NET=y
CONFIG_UNIX=y
CONFIG_INET=y
CONFIG_IPV6=y
CONFIG_PACKET=y
CONFIG_NETFILTER=y

CONFIG_VFAT_FS=y
CONFIG_NLS=y
CONFIG_NLS_DEFAULT="utf8"
CONFIG_NLS_CODEPAGE_437=y
CONFIG_NLS_ISO8859_1=y

CONFIG_EXT4_FS=y

CONFIG_FSNOTIFY=y
CONFIG_DNOTIFY=y
CONFIG_INOTIFY_USER=y

# virtio-network
CONFIG_PCI=y
CONFIG_PCI_MSI=y
CONFIG_VIRTIO_MENU=y
CONFIG_VIRTIO=y
CONFIG_VIRTIO_PCI=y
CONFIG_VIRTIO_NET=y
CONFIG_NETDEVICES=y
CONFIG_NET_CORE=y

# virtio-blk
CONFIG_PCI=y
CONFIG_PCI_MSI=y
CONFIG_BLK_DEV=y
CONFIG_VIRTIO_MENU=y
CONFIG_VIRTIO=y
CONFIG_VIRTIO_PCI=y
CONFIG_VIRTIO_BLK=y

# vritio-fs
CONFIG_MEMORY_HOTPLUG=y
CONFIG_MEMORY_HOTREMOVE=y
CONFIG_FUSE_FS=y
CONFIG_PCI=y
CONFIG_PCI_MSI=y
CONFIG_VIRTIO_MENU=y
CONFIG_VIRTIO=y
CONFIG_VIRTIO_PCI=y
CONFIG_VIRTIO_FS=y
CONFIG_DAX=y
CONFIG_FS_DAX=y
CONFIG_ZONE_DEVICE=y
```

# License

[GPL 3.0](COPYING)

