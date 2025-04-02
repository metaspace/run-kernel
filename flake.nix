{
  description = "Tool for running kernel images";
  inputs.nixpkgs.url = "nixpkgs/nixos-24.11";
  inputs.systems.url = "github:nix-systems/default";
  inputs.flake-utils = {
    url = "github:numtide/flake-utils";
    inputs.systems.follows = "systems";
  };
  inputs.rust-overlay = {
    url = "github:oxalica/rust-overlay";
    inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ rust-overlay.overlays.default ];
        pkgs = import nixpkgs { inherit system overlays; };
        toolchainStatic = pkgs.rust-bin.stable.latest.minimal.override {
          targets = [ "x86_64-unknown-linux-musl" ];
        };
        rustPlatformStatic = pkgs.makeRustPlatform {
          cargo = toolchainStatic;
          rustc = toolchainStatic;
        };
      in {
        devShells.default =
          pkgs.mkShell { packages = [ pkgs.gnumake pkgs.pkg-config ]; };
        packages.default = let
          runKernelInit = rustPlatformStatic.buildRustPackage {
            pname = "run-kernel-init";
            version = "0.1.0";
            src = ./init;
            cargoLock = { lockFile = ./init/Cargo.lock; };
            doCheck = false;
            nativeBuildInputs = [ pkgs.gnumake pkgs.pkg-config pkgs.perl ];
            buildPhase = ''
              cargo build --target x86_64-unknown-linux-musl --release
            '';
            installPhase = ''
              mkdir -p $out/bin
              cp target/x86_64-unknown-linux-musl/release/init $out/bin/init
            '';
          };
        in pkgs.rustPlatform.buildRustPackage {
          pname = "run-kernel";
          version = "0.1.0";
          src = ./.;
          cargoLock = { lockFile = ./Cargo.lock; };
          doCheck = false;
          nativeBuildInputs = [ pkgs.gnumake pkgs.pkg-config pkgs.perl ];
          RUN_KERNEL_INIT_PATH = runKernelInit + /bin/init;
        };
      });
}
