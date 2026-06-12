{
  config,
  pkgs,
  lib,
  gtkshutdown,
  ...
}: let
  openApps = pkgs.writeShellApplication {
    name = "open_apps";
    runtimeInputs = with pkgs; [
      kitty
      firefox
      thunar
    ];

    text = ''
      firefox &
      kitty &
      thunar &
    '';
  };
in {
  environment.systemPackages = [
    openApps
    (pkgs.writeShellScriptBin "gtkshutdown" "RUST_LOG=trace ${lib.getExe gtkshutdown}")
  ];

  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;
  fileSystems."/" = {
    device = "/dev/disk/by-label/nixos"; # Will be overridden automatically by qemu-vm
    fsType = "ext4";
  };

  users.users.guest = {
    description = "Guest";
    hashedPassword = "";
    isNormalUser = true;
    extraGroups = ["wheel"];
  };

  security.sudo.wheelNeedsPassword = false;

  services.getty.autologinUser = "guest";

  # The state version can safely track the latest release because the disk
  # image is ephemeral.
  system.stateVersion = config.system.nixos.release;
  home-manager.users.guest.home.stateVersion =
    config.system.nixos.release;

  home-manager.sharedModules = lib.singleton {
    # Enable Bash to ensure environment variables are set.
    programs.bash.enable = true;
  };

  virtualisation.vmVariant.virtualisation = {
    cores = 4; # This is a maximum limit; the VM should still work if the host has fewer cores
    memorySize = lib.mkDefault 2048;
  };
}
