{
  description = "Tool for running kernel images";
  inputs.nixpkgs.url = "nixpkgs/nixos-24.05";
  inputs.systems.url = "github:nix-systems/default";
  inputs.flake-utils = {
    url = "github:numtide/flake-utils";
    inputs.systems.follows = "systems";
  };

  outputs =
    { nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell { packages = [
          pkgs.gnumake
          pkgs.pkg-config
        ]; };
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "run-kernel";
          version = "0.1.0";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          doCheck = false;
          nativeBuildInputs = [
            pkgs.gnumake
            pkgs.pkg-config
            pkgs.perl
          ];
        };
      }
    );
}
