{
  description = "Devshell for this project";

  nixConfig = {
    extra-substituters = [
      "https://fenix.cachix.org"
    ];
    extra-trusted-public-keys = [
      "fenix.cachix.org-1:ecJhr+RdYEdcVgUkjruiYhjbBloIEGov7bos90cZi0Q="
    ];
  };

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    fenix.url = "github:nix-community/fenix";

    git-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    home-manager = {
      url = "github:nix-community/home-manager/release-26.05";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    home-manager,
    ...
  } @ inputs: let
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
      overlays = [inputs.fenix.overlays.default];
    };
    lib = pkgs.lib;

    rust-toolchain = pkgs.fenix.fromToolchainFile {
      file = ./rust-toolchain.toml;
      sha256 = "sha256-mvUGEOHYJpn3ikC5hckneuGixaC+yGrkMM/liDIDgoU=";
    };

    rustPlatform = pkgs.makeRustPlatform {
      cargo = rust-toolchain;
      rustc = rust-toolchain;
    };

    gtkshutdown = pkgs.callPackage ./nix {inherit rustPlatform;};

    pre-commit-check = (import ./nix/dev/pre-commit.nix) {inherit inputs system rust-toolchain;};
    devshell = (import ./nix/dev/devshell.nix) {inherit pkgs rust-toolchain pre-commit-check;};

    mkVm = name:
      nixpkgs.lib.nixosSystem {
        inherit system;
        specialArgs = {inherit gtkshutdown;};

        modules = [
          inputs.home-manager.nixosModules.home-manager
          ./nix/dev/vms/${name}.nix
        ];
      };
  in {
    nixosConfigurations = {
      hyprland = mkVm "hyprland";
    };

    apps.${system} = let
      mkVmApp = name: vmConfig: {
        type = "app";
        program = "${lib.getExe (pkgs.writeShellApplication {
          inherit name;
          runtimeInputs = [pkgs.coreutils];
          text = ''
            # Remove the disk image file after running the vm, since it isn't
            # needed
            cleanup() {
              if rm --recursive "$directory"; then
                printf '%s\n' 'Virtualisation disk image removed.'
              fi
            }

            trap cleanup EXIT

            # We create a temporary directory rather than a temporary file, since
            # temporary files are created empty and are not valid disk images.
            directory="$(mktemp --directory)"

            NIX_DISK_IMAGE="$directory/nixos.qcow2" \
              ${lib.getExe vmConfig.config.system.build.vm}
          '';
        })}";
      };
    in
      builtins.mapAttrs (name: value: mkVmApp name value) self.nixosConfigurations;

    packages.${system}.default = gtkshutdown;
    devShells.${system}.default = devshell;
  };
}
