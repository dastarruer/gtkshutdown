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
  };

  outputs = {
    self,
    nixpkgs,
    ...
  } @ inputs: let
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
      overlays = [inputs.fenix.overlays.default];
    };

    rust-toolchain = pkgs.fenix.fromToolchainFile {
      file = ./rust-toolchain.toml;
      sha256 = "sha256-Qxt8XAuaUR2OMdKbN4u8dBJOhSHxS+uS06Wl9+flVEk=";
    };

    rustPlatform = pkgs.makeRustPlatform {
      cargo = rust-toolchain;
      rustc = rust-toolchain;
    };

    gtkshutdown = pkgs.callPackage ./nix {inherit rustPlatform;};

    pre-commit-check = (import ./nix/dev/pre-commit.nix) {inherit inputs system rust-toolchain;};
    devshell = (import ./nix/dev/devshell.nix) {inherit pkgs rust-toolchain pre-commit-check;};
  in {
    packages.${system}.default = gtkshutdown;
    devShells.${system}.default = devshell;
  };
}
