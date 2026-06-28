{
  pkgs,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "gtkshutdown";
  version = "0.1.0";
  src = ../.;

  cargoLock = {
    lockFile = ../Cargo.lock;
    outputHashes = {
      "hyprland-0.4.0-beta.3" = "sha256-8rOAx9Hndezc7zQzIs/Z0GT77iDslKmAU9tzfOusH74=";
    };
  };

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

  # A janky workaround to get gtkshutdown to recognize build inputs
  postFixup = ''
    wrapProgram $out/bin/gtkshutdown \
      --prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath buildInputs}"
  '';

  meta = with pkgs.lib; {
    description = "A smooth application closer utility for Hyprland/Wayland ecosystems";
    homepage = "https://github.com/dastarruer/gtkshutdown";
    license = licenses.bsd3;
    mainProgram = "gtkshutdown";
    platforms = platforms.linux;
  };
}
