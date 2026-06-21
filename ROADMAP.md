# Orbit Ecosystem Roadmap

The Orbit Window Manager is designed to bring a true Arch Linux (Hyprland / DWM) dynamic tiling experience to Windows 11 with zero compromises in performance and extreme customizability.

## Phase 1: Core Tiling & UI Foundations (✅ Completed)
- [x] **Win32 Event Hooks:** Zero-overhead background window management.
- [x] **Dynamic Layouts:** Hyprland-style Dwindle Layout and classic DWM Master-Stack Layout.
- [x] **Virtual Desktops:** Native Windows 11 Virtual Workspace tracking via `IVirtualDesktopManager`.
- [x] **IPC Telemetry:** Real-time UDP broadcasting of window states.
- [x] **OrbitBar Frontend:** Waybar-inspired status bar built in Rust/egui.
- [x] **Native Shell Integration:** AppBar registration (`ABM_NEW`, `ABM_SETPOS`) to prevent OS minimization (`Win+D`) and overlap.
- [x] **Hot-Reloading:** Dynamic TOML configs (`orbit.toml`, `orbitbar.toml`) that update instantly on save.

## Phase 2: Animations & Visual Polish
- [ ] **Bezier Curve Window Physics:** Implement 60FPS fluid sliding animations when windows spawn, close, or swap positions, replacing instant snapping.
- [ ] **Window Borders:** Inject custom colored borders around the currently focused window.
- [ ] **Window Gaps Polish:** Fix any remaining edge-case calculation bugs with inner and outer gaps.
- [ ] **Blur Effects:** Add background acrylic/mica blur capabilities to OrbitBar.

## Phase 3: Advanced Layouts & Monitors
- [ ] **Multi-Monitor Support:** Intercept `EnumDisplayMonitors` to give each physical display its own independent layout and workspace array.
- [ ] **Window Rules:** Add regex rules to `orbit.toml` to force certain apps (like Steam, popups, or Picture-in-Picture) to always float and never tile.
- [ ] **Floating Mode Toggle:** Allow users to instantly pop a window out of the grid and drag it freely.
- [ ] **Monocle / Fullscreen Mode:** Allow a window to temporarily hide OrbitBar and take up 100% of the screen.

## Phase 4: OrbitBar Expansion
- [ ] **Interactive Modules:** Make OrbitBar clickable (e.g., clicking the Workspace 2 icon forces Windows to switch to Virtual Desktop 2).
- [ ] **System Tray Porting:** Extract the native Windows System Tray (Wi-Fi, Volume, Bluetooth) and render it inside an OrbitBar module.
- [ ] **Battery & Media Modules:** Add native battery tracking and currently playing media modules.
- [ ] **CSS Engine:** Expand customization by allowing users to load a `style.css` file to heavily modify pill shapes, gradients, and fonts, similar to Waybar.

## Phase 5: Keyboard Control Daemon
- [ ] **OrbitHK (Hotkey Engine):** Implement a global keyboard hook system directly into Orbit to handle layout commands.
- [ ] **Directional Focus:** Shortcuts to focus windows via directional keys (Win + H/J/K/L).
- [ ] **Window Swapping:** Shortcuts to physically swap the position of two tiled windows.
