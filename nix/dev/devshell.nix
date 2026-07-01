{
  pkgs,
  rust-toolchain,
  pre-commit-check,
}:
pkgs.mkShell rec {
  packages = with pkgs; [
    # MARKDOWN
    markdownlint-cli
    prettier

    # NIX
    nixd
    alejandra

    # RUST
    rust-toolchain
  ];

  nativeBuildInputs = with pkgs; [
    pkg-config
    glib
    wrapGAppsHook4
  ];

  buildInputs = with pkgs; [
    gtk4
    pango
    glib
  ];

  env.LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;

  shellHook = ''
    # Install pre-commit hooks
    ${pre-commit-check.shellHook}
  '';
}
