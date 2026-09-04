{
  pkgs,
  lib,
  ...
}: {
  imports = [
    ./common.nix
  ];

  environment.loginShellInit = lib.getExe' pkgs.mango "mango";

  # mango requires GPU rendering, so these settings are required
  hardware.graphics.enable = true;
  virtualisation.vmVariant.virtualisation.qemu.options = [
    "-vga"
    "none"
    "-device"
    "virtio-vga"
  ];

  programs.mango.enable = true;
  environment.etc."mango/config.conf".text = ''
    bind=SUPER,GRAVE,spawn,kitty
  '';
}
