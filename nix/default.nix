{
  pkgs,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
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
}
