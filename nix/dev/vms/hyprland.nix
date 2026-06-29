{
  pkgs,
  lib,
  ...
}: {
  imports = [
    ./common.nix
  ];

  environment.loginShellInit = lib.getExe' pkgs.hyprland "start-hyprland";
  programs.hyprland.enable = true;
  environment.systemPackages = [
    # dex looks for `x-terminal-emulator` when running a terminal program
    (pkgs.writeShellScriptBin "x-terminal-emulator" ''exec ${lib.getExe pkgs.kitty} "$@"'')
  ];

  home-manager.sharedModules = lib.singleton {
    wayland.windowManager.hyprland = {
      enable = true;
      systemd.enable = true;
      configType = "hyprlang";

      settings = {
        exec-once = [
          "find /run/current-system/sw/etc/xdg/autostart/ -type f -or -type l | xargs -P0 -L1 ${lib.getExe pkgs.dex}"
        ];

        ecosystem = {
          no_update_news = true;
          no_donation_nag = true;
        };

        bind = [
          # The backtick (`) key
          "SUPER, code:49, exec, ${lib.getExe pkgs.kitty} "
        ];
      };
    };
  };
}
