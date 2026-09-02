{
  pkgs,
  inputs,
  gtkshutdown,
  ...
}:
pkgs.testers.runNixOSTest {
  name = "gtkshutdown-test";

  node.specialArgs = {inherit gtkshutdown;};
  nodes.machine = {...}: {
    imports = [
      ../vms/hyprland.nix
      inputs.home-manager.nixosModules.home-manager
    ];

    virtualisation = {
      cores = 4;
      # Might crash with less
      memorySize = 8192;
      resolution = {
        x = 1920;
        y = 1080;
      };

      qemu.options = ["-vga none -device virtio-gpu-pci"];
    };
  };

  testScript = ''
    marker = "/tmp/post-cmd-ran"
    uid = "1000"

    start_all()
    machine.wait_for_unit("multi-user.target")

    # Wait for the compositor's Wayland socket to exist before launching GUI apps.
    machine.wait_for_file(f"/run/user/{uid}/wayland-1")

    def run_as_guest(cmd):
        return machine.succeed(
            f"su - guest -c 'WAYLAND_DISPLAY=wayland-1 "
            f"XDG_RUNTIME_DIR=/run/user/{uid} {cmd}'"
        )

    # Launch the apps
    run_as_guest("open_apps >&2 &")

    print("Waiting for apps to open")
    for proc in ["firefox", "kitty", "thunar", "waybar"]:
        machine.wait_until_succeeds(f"pgrep -f {proc}")

    print("Starting gtkshutdown")
    run_as_guest(f'gtkshutdown --post-cmd "touch {marker}" --no-fork')

    print("Checking that apps are closed")
    for proc in ["firefox", "kitty", "thunar", "waybar"]:
        machine.wait_until_fails(f"pgrep -f {proc}")

    print("Checking that post-cmd executed successfully")
    machine.wait_for_file(marker)

    machine.shutdown()
  '';
}
