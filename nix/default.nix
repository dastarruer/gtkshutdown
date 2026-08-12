{
  inputs,
  pkgs,
  toolchain,
}: let
  craneLib = (inputs.crane.mkLib inputs.nixpkgs.legacyPackages.${pkgs.stdenv.hostPlatform.system})
    .overrideToolchain toolchain;

  src = pkgs.lib.cleanSourceWith {
    src = ../.;
    filter = path: type: let
      name = baseNameOf path;
    in
      craneLib.filterCargoSources path type
      || name == "style.css"; # Keep style.css; filter out all other unnecessary files
    name = "source";
  };

  commonArgs = {
    nativeBuildInputs = [
      pkgs.pkg-config
    ];

    buildInputs = with pkgs; [
      gtk4
      glib
      pango
    ];
  };

  cargoArtifacts = craneLib.buildDepsOnly (commonArgs
    // {
      inherit src;
    });
in
  craneLib.buildPackage (commonArgs
    // {
      inherit src cargoArtifacts;

      nativeBuildInputs =
        commonArgs.nativeBuildInputs
        ++ [
          pkgs.wrapGAppsHook4
        ];

      cargoExtraArgs = "-p gtkshutdown";

      postInstall = ''
        if [ -f "$out/bin/gtkshutdown" ]; then
          wrapGApp $out/bin/gtkshutdown \
            --prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath commonArgs.buildInputs}"
        fi
      '';

      meta = with pkgs.lib; {
        description = "A graceful shutdown utility for Wayland window managers/compositors.";
        homepage = "https://github.com/dastarruer/gtkshutdown";
        license = licenses.bsd3;
        mainProgram = "gtkshutdown";
        platforms = platforms.linux;
      };
    })
