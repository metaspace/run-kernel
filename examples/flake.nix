{
  description = "A system expression for run-kernel";
  inputs = { nixpkgs.url = "nixpkgs/nixos-24.11"; };

  outputs = { nixpkgs, ... }: {
    nixosConfigurations.vm = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [ ./configuration.nix ];
    };
  };
}
