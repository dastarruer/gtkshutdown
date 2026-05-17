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

    pre-commit-check = inputs.git-hooks.lib.${system}.run {
      src = ./.;

      # GIT HOOKS GO HERE
      # See https://devenv.sh/git-hooks/ for how to configure hooks
      # To get the root of the project, use the following command as a workaround: $(git rev-parse --show-toplevel)
      # See https://github.com/NixOS/nix/issues/8034#issuecomment-3366842508 for more info
      hooks = {
        alejandra.enable = true;

        clippy = {
          enable = true;

          packageOverrides = {
            cargo = rust-toolchain;
            clippy = rust-toolchain;
          };

          settings = {
            allFeatures = true;
            denyWarnings = true;
          };
        };

        rustfmt = {
          enable = true;

          packageOverrides = {
            cargo = rust-toolchain;
            rustfmt = rust-toolchain;
          };

          settings.check = true;
        };

        check-toml.enable = true;
        taplo.enable = true;

        markdownlint.enable = true;
      };
    };
  in {
    devShells.${system}.default = pkgs.mkShell {
      nativeBuildInputs = with pkgs; [
        # MARKDOWN
        markdownlint-cli # Formatter

        # NIX
        nixd # LSP
        alejandra # Formatter

        # RUST
        rust-toolchain
        rust-analyzer-nightly
        pkg-config
        gtk4
      ];

      shellHook = ''
        # Install pre-commit hooks
        ${pre-commit-check.shellHook}
      '';
    };
  };
}
