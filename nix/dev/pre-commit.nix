{
  inputs,
  system,
  rust-toolchain,
}:
inputs.git-hooks.lib.${system}.run {
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
    };

    check-toml.enable = true;
    taplo.enable = true;

    prettier = {
      enable = true;
      settings.configPath = ".prettierrc";
    };
    markdownlint.enable = true;
  };
}
