# Orbit
A highly customizable, dynamic tiling window manager for Windows 11, inspired by Arch Linux's Hyprland.

## Features
*   **True DWM Native Interpolation:** Hooks directly into the OS event loop.
*   **Dynamic Tiling:** Includes Hyprland's signature recursive Dwindle Layout and the classic Master-Stack Layout.
*   **Virtual Desktops:** Natively respects Windows 11 virtual workspaces with buttery-smooth sliding animations.
*   **Zero-Overhead Interop:** Near 0% CPU footprint using native Win32 hooks.
*   **Hot-Reloading Configurations:** Changes to `orbit.toml` apply instantly without restarting.
*   **AppBar Integration:** Modifies the OS Work Area (`SPI_SETWORKAREA`) to natively prevent maximized windows from covering your bar.

## Installation
Ensure you have Rust installed, then run:
```bash
cargo build --release
```
Run `target/release/Orbit.exe`.

## Configuration
Edit `orbit.toml` in the executable's directory. 
Example:
```toml
[layout]
gap_size = 12
master_ratio = 0.5
layout_type = "dwindle" # or "master"
```

## Telemetry
Orbit broadcasts window count and active titles via UDP on `127.0.0.1:8123`.

## License
[GPL-3.0 License](LICENSE)

## To-Do
*   Window physics and bezier animation curves.
*   Multi-monitor (`Monitors` abstraction layer) support.
