{
  pkgs,
  lib,
  ...
}: {
  imports = [
    ./common.nix
  ];

  environment.loginShellInit = lib.getExe' pkgs.sway "sway";
  programs.sway.enable = true;
  environment.systemPackages = [
    # dex looks for `x-terminal-emulator` when running a terminal program
    (pkgs.writeShellScriptBin "x-terminal-emulator" ''exec ${lib.getExe pkgs.kitty} "$@"'')
  ];

  home-manager.sharedModules = lib.singleton {
    wayland.windowManager.sway = {
      enable = true;
      config = {
        startup = [
          {command = "find /run/current-system/sw/etc/xdg/autostart/ -type f -or -type l | xargs -P0 -L1 ${lib.getExe pkgs.dex}";}
        ];

        keybindings = {
          # The backtick (`) key
          "Mod4+Grave" = "exec ${lib.getExe pkgs.kitty}";
        };
      };
    };
  };
}
