{ pkgs, lib, modulesPath, ... }: {
  imports = [ (modulesPath + "/profiles/minimal.nix") ];
  options = { };
  config = {

    # We are stateless, so we don't use this, but nix will warn without it.
    system.stateVersion = "24.11";

    # Shut up nixos warning about missing root
    fileSystems."/" = lib.mkImageMediaOverride {
      fsType = "tmpfs";
      options = [ "mode=0755" ];
    };

    # Disable the remount rw systemd service
    systemd.services.systemd-remount-fs.enable = lib.mkForce false;

    # Don't build the GRUB menu builder script, since we don't need it
    # here and it causes a cyclic dependency.
    boot.loader.grub.enable = false;

    # No need to build kernel or initrd
    boot.kernel.enable = false;
    boot.initrd.enable = false;

    networking.hostName = "";
    #networking.dhcpcd.enable = false;

    # Uncomment if you want log output on console
    #services.journald.console = "/dev/console";

    # The system is static.
    users.mutableUsers = false;

    # Empty password for root
    users.users.root.initialHashedPassword = "";

    # Log in root automatically
    services.getty.autologinUser = "root";

    # Disable the oom killer
    systemd.oomd.enable = false;

    # Disable firewall
    networking.firewall.enable = false;

    # The system cannot be rebuilt
    nix.enable = false;

    # No logical volume management
    services.lvm.enable = false;

    # Enable ssh and allow root login without password
    services.sshd.enable = true;
    services.openssh.settings.PermitRootLogin = "yes";
    services.openssh.settings.PermitEmptyPasswords = "yes";
    security.pam.services.sshd.allowNullPassword = true;

    # For mounting virtiofs passed to run-kernel via --share share:/path/to/place
    fileSystems."/mnt" = {
      fsType = "virtiofs";
      device = "share";
    };

    environment.systemPackages = [ pkgs.coreutils pkgs.python3 pkgs.wget ];
  };
}
