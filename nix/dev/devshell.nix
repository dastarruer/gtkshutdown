{
  pkgs,
  rust-toolchain,
  pre-commit-check,
}:
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    # MARKDOWN
    markdownlint-cli # Linter
    prettier

    # NIX
    nixd # LSP
    alejandra # Formatter

    # RUST
    rust-toolchain
    pkg-config
    gtk4
  ];

  shellHook = ''
    # Install pre-commit hooks
    ${pre-commit-check.shellHook}
  '';
}
