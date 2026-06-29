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
    programs.kitty.enable = true;

    wayland.windowManager.hyprland = {
      enable = true;
      systemd.enable = true;
      configType = "lua";

      settings = {
        on = {
          _args = [
            "hyprland.start"
            (lib.generators.mkLuaInline ''
              function()
                hl.exec_cmd("find /run/current-system/sw/etc/xdg/autostart/ -type f -or -type l | xargs -P0 -L1 ${lib.getExe pkgs.dex}")
              end'')
          ];
        };

        config.ecosystem = {
          no_update_news = true;
          no_donation_nag = true;
        };

        bind = [
          # The backtick (`) key
          {_args = ["SUPER + code:49" (lib.generators.mkLuaInline ''hl.dsp.exec_cmd("${lib.getExe pkgs.kitty}")'')];}
        ];
      };
    };
  };
}
