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

    gtkshutdown = rustPlatform.buildRustPackage {
      pname = "gtkshutdown";
      version = "0.1.0";
      src = ./.;

      cargoLock = {
        lockFile = ./Cargo.lock;
        outputHashes = {
          "hyprland-0.4.0-beta.3" = "sha256-8rOAx9Hndezc7zQzIs/Z0GT77iDslKmAU9tzfOusH74=";
        };
      };

      nativeBuildInputs = with pkgs; [
        pkg-config
        wrapGAppsHook4
      ];

      buildInputs = with pkgs; [
        gtk4
      ];

      meta = with pkgs.lib; {
        description = "A smooth application closer utility for Hyprland/Wayland ecosystems";
        homepage = "https://github.com/dastarruer/gtkshutdown";
        license = licenses.bsd3;
        mainProgram = "gtkshutdown";
        platforms = platforms.linux;
      };
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

        prettier = {
          enable = true;
          settings.configPath = ".prettierrc";
        };
        markdownlint.enable = true;
      };
    };
  in {
    packages.${system} = {
      default = gtkshutdown;
      gtkshutdown = gtkshutdown;
    };

    devShells.${system}.default = pkgs.mkShell {
      nativeBuildInputs = with pkgs; [
        # MARKDOWN
        markdownlint-cli # Linter
        prettier

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
