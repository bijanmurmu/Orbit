use std::sync::Mutex;
use std::process::Command;
use std::os::windows::process::CommandExt;
use windows::Win32::Foundation::{HWND, RECT, LPARAM, WPARAM};
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, MOD_WIN, MOD_ALT, VK_RETURN};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, GetWindowLongW, IsWindowVisible, GetWindowRect, EnumWindows,
    EVENT_OBJECT_CREATE, EVENT_OBJECT_DESTROY, EVENT_OBJECT_SHOW, EVENT_SYSTEM_FOREGROUND, MSG,
    GWL_STYLE, GWL_EXSTYLE, WS_CHILD, WS_EX_TOOLWINDOW, WM_HOTKEY, WM_CLOSE, PostQuitMessage,
    WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, GetWindowTextLengthW, GetWindowTextW,
    SetWindowPos, SystemParametersInfoW, SPI_GETWORKAREA, SPI_SETWORKAREA, SPIF_SENDCHANGE, SWP_NOACTIVATE, SWP_NOZORDER, IsWindow,
    ShowWindow, SW_RESTORE, GetWindow, GW_OWNER, WS_MAXIMIZE, WS_MINIMIZE, GetForegroundWindow, SendMessageW,
    SetWindowLongW, GWLP_HWNDPARENT, HWND_TOPMOST, SWP_SHOWWINDOW, GetSystemMetrics, SM_CXSCREEN, EVENT_SYSTEM_DESKTOPSWITCH, EVENT_OBJECT_HIDE
};
use windows::Win32::UI::Shell::{IVirtualDesktopManager, VirtualDesktopManager};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER, CoInitializeEx, COINIT_MULTITHREADED};
use std::net::UdpSocket;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use notify::{Watcher, RecursiveMode, Result as NotifyResult};

static WORKSPACE: Mutex<Vec<isize>> = Mutex::new(Vec::new());

#[derive(Deserialize, Debug, Clone)]
struct LayoutConfig {
    gap_size: i32,
    master_ratio: f32,
    layout_type: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
struct Config {
    layout: LayoutConfig,
}

thread_local! {
    static VDM: std::cell::RefCell<Option<IVirtualDesktopManager>> = std::cell::RefCell::new({
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();
            CoCreateInstance(&VirtualDesktopManager, None, CLSCTX_INPROC_SERVER).ok()
        }
    });
}

impl Default for Config {
    fn default() -> Self {
        Self {
            layout: LayoutConfig { gap_size: 12, master_ratio: 0.5, layout_type: Some("dwindle".to_string()) },
        }
    }
}

static CONFIG: Mutex<Option<Config>> = Mutex::new(None);

fn get_config() -> Config {
    let lock = CONFIG.lock().unwrap();
    if let Some(c) = &*lock {
        c.clone()
    } else {
        Config::default()
    }
}

fn load_config() {
    let path = "orbit.toml";
    if let Ok(contents) = fs::read_to_string(path) {
        if let Ok(config) = toml::from_str::<Config>(&contents) {
            *CONFIG.lock().unwrap() = Some(config);
            update_layout();
        }
    }
}

fn watch_config() {
    std::thread::spawn(|| {
        let mut watcher = notify::recommended_watcher(|res: NotifyResult<notify::Event>| {
            match res {
                Ok(event) => {
                    if event.kind.is_modify() {
                        load_config();
                    }
                },
                Err(e) => println!("watch error: {:?}", e),
            }
        }).unwrap();
        
        let _ = watcher.watch(Path::new("orbit.toml"), RecursiveMode::NonRecursive);
        loop { std::thread::park(); }
    });
}

const HOTKEY_QUIT: i32 = 1;
const HOTKEY_TERM: i32 = 2;
const HOTKEY_CLOSE: i32 = 3;

fn register_hotkeys() {
    unsafe {
        // Win + Alt + Q -> Quit Orbit
        if RegisterHotKey(None, HOTKEY_QUIT, MOD_WIN | MOD_ALT, 0x51).is_err() {
            println!("[-] Failed to register Win+Alt+Q");
        }
        // Win + Alt + Enter -> Open Terminal
        if RegisterHotKey(None, HOTKEY_TERM, MOD_WIN | MOD_ALT, VK_RETURN.0 as u32).is_err() {
            println!("[-] Failed to register Win+Alt+Enter");
        }
        // Win + Alt + C -> Close active window
        if RegisterHotKey(None, HOTKEY_CLOSE, MOD_WIN | MOD_ALT, 0x43).is_err() {
            println!("[-] Failed to register Win+Alt+C");
        }
    }
}

fn set_window_geometry(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
    unsafe {
        // SMOOTH COMPOSITING: In a real environment we'd use a timer/hook loop.
        // For absolute Arch perfection + speed, SWP_NOACTIVATE ensures fast DWM native transitions.
        let _ = SetWindowPos(hwnd, None, x, y, w, h, SWP_NOACTIVATE | SWP_NOZORDER);
        
        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        if (style & WS_MAXIMIZE.0) != 0 || (style & WS_MINIMIZE.0) != 0 {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }

        let mut frame: RECT = std::mem::zeroed();
        let res = DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, &mut frame as *mut _ as *mut _, std::mem::size_of::<RECT>() as u32);
        
        let mut raw: RECT = std::mem::zeroed();
        let _ = GetWindowRect(hwnd, &mut raw);

        // Prevent invalid dimensions
        let safe_w = w.max(10);
        let safe_h = h.max(10);

        // 0x0020 is SWP_FRAMECHANGED - Forces the window to recalculate its client area (fixes blank Chrome)
        let flags = SWP_NOACTIVATE | SWP_NOZORDER | windows::Win32::UI::WindowsAndMessaging::SWP_FRAMECHANGED;

        if res.is_ok() {
            let border_left = frame.left - raw.left;
            let border_top = frame.top - raw.top;
            let border_right = raw.right - frame.right;
            let border_bottom = raw.bottom - frame.bottom;

            let _ = SetWindowPos(
                hwnd, None, 
                x - border_left, 
                y - border_top, 
                safe_w + border_left + border_right, 
                safe_h + border_top + border_bottom, 
                flags
            );
        } else {
            let _ = SetWindowPos(hwnd, None, x, y, safe_w, safe_h, flags);
        }
    }
}

fn update_layout() {
    let mut hwnds = WORKSPACE.lock().unwrap();
    hwnds.retain(|&h| unsafe { IsWindow(Some(HWND(h as *mut _))).as_bool() });
    if hwnds.is_empty() { return; }

    unsafe {
        let mut work_area: RECT = std::mem::zeroed();
        let _ = SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut work_area as *mut _ as *mut _), Default::default());

        // Reserve 34 pixels at the top for OrbitBar so windows never overlap it
        work_area.top += 34;

        let width = work_area.right - work_area.left;
        let height = work_area.bottom - work_area.top;
        let n = hwnds.len() as i32;
        if n == 0 { return; }

        let config = get_config();
        let gap = config.layout.gap_size;

        if n == 1 {
            let hwnd = HWND(hwnds[0] as *mut _);
            set_window_geometry(
                hwnd, 
                work_area.left + gap, 
                work_area.top + gap, 
                width - (gap * 2), 
                height - (gap * 2)
            );
        } else {
            let layout_type = config.layout.layout_type.unwrap_or_else(|| "master".to_string());
            
            if layout_type == "dwindle" {
                // HYPRLAND SIGNATURE: Recursive Dwindle Layout
                let mut cur_x = work_area.left + gap;
                let mut cur_y = work_area.top + gap;
                let mut cur_w = width - (gap * 2);
                let mut cur_h = height - (gap * 2);

                for i in 0..n {
                    let hwnd = HWND(hwnds[i as usize] as *mut _);
                    let is_last = i == n - 1;
                    let split_vertically = cur_w > cur_h;

                    let (win_w, win_h) = if is_last {
                        (cur_w, cur_h)
                    } else if split_vertically {
                        ((cur_w - gap) / 2, cur_h)
                    } else {
                        (cur_w, (cur_h - gap) / 2)
                    };

                    set_window_geometry(hwnd, cur_x, cur_y, win_w, win_h);

                    if !is_last {
                        if split_vertically {
                            cur_x += win_w + gap;
                            cur_w -= win_w + gap;
                        } else {
                            cur_y += win_h + gap;
                            cur_h -= win_h + gap;
                        }
                    }
                }
            } else {
                // CLASSIC DWM: Master-Stack Layout
                let master_w = (width as f32 * config.layout.master_ratio) as i32;
                let final_master_w = master_w - (gap as f32 * 1.5) as i32;
                
                let master_hwnd = HWND(hwnds[0] as *mut _);
                set_window_geometry(
                    master_hwnd, 
                    work_area.left + gap, 
                    work_area.top + gap, 
                    final_master_w, 
                    height - (gap * 2)
                );

                let stack_x = work_area.left + gap + final_master_w + gap;
                let stack_w = width - final_master_w - (gap * 3);
                let stack_h = (height - (gap * (n))) / (n - 1);

                for i in 1..n {
                    let hwnd = HWND(hwnds[i as usize] as *mut _);
                    let stack_y = work_area.top + gap + (i - 1) * (stack_h + gap);
                    set_window_geometry(hwnd, stack_x, stack_y, stack_w, stack_h);
                }
            }
        }
    }

    broadcast_telemetry();
}

fn is_manageable(hwnd: HWND) -> bool {
    unsafe {
        let mut buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut buf);
        if len > 0 {
            let title = String::from_utf16_lossy(&buf[..len as usize]);
            if title == "OrbitBar" {
                static mut REGISTERED: bool = false;
                if !REGISTERED {
                    let w = GetSystemMetrics(SM_CXSCREEN);
                    let h = 36; // OrbitBar height

                    let exstyle = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
                    let _ = SetWindowLongW(hwnd, GWL_EXSTYLE, (exstyle | WS_EX_TOOLWINDOW.0) as i32);
                    
                    // Detach completely from any owner window so it never minimizes with them
                    #[cfg(target_pointer_width = "64")]
                    unsafe { windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, 0) };
                    #[cfg(target_pointer_width = "32")]
                    unsafe { SetWindowLongW(hwnd, GWLP_HWNDPARENT, 0) };

                    let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, w, h, SWP_SHOWWINDOW | SWP_NOACTIVATE);

                    // HARD FIX: Modify the OS Work Area directly so Chrome NEVER maximizes over us
                    let mut work_area: RECT = std::mem::zeroed();
                    let _ = SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut work_area as *mut _ as *mut _), Default::default());
                    if work_area.top < h {
                        work_area.top = h;
                        let _ = SystemParametersInfoW(SPI_SETWORKAREA, 0, Some(&mut work_area as *mut _ as *mut _), SPIF_SENDCHANGE);
                    }

                    REGISTERED = true;
                }
                
                // HARD FIX: If Win+D tries to hide it, violently force it back
                let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
                if (style & WS_MINIMIZE.0) != 0 || !IsWindowVisible(hwnd).as_bool() {
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                }

                return false; 
            }
        }

        if !IsWindowVisible(hwnd).as_bool() {
            return false;
        }

        let mut on_current = true;
        VDM.with(|vdm| {
            if let Some(desktop_manager) = &*vdm.borrow() {
                if let Ok(is_current) = desktop_manager.IsWindowOnCurrentVirtualDesktop(hwnd) {
                    on_current = is_current.as_bool();
                }
            }
        });

        if !on_current {
            return false;
        }

        let owner = GetWindow(hwnd, GW_OWNER);
        if owner.is_ok() {
            return false;
        }

        let mut cloaked: u32 = 0;
        let _ = DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut _ as *mut _, std::mem::size_of::<u32>() as u32);
        if cloaked != 0 { return false; }

        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;

        if (style & WS_CHILD.0) != 0 { return false; }
        if (ex_style & WS_EX_TOOLWINDOW.0) != 0 { return false; }

        true
    }
}

unsafe fn get_window_title(hwnd: HWND) -> String {
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len == 0 { return String::new(); }
    let mut buf: Vec<u16> = vec![0; (len + 1) as usize];
    unsafe { GetWindowTextW(hwnd, &mut buf) };
    String::from_utf16_lossy(&buf).trim_end_matches('\0').to_string()
}

fn broadcast_telemetry() {
    unsafe {
        let hwnd = GetForegroundWindow();
        let mut title = String::new();
        if hwnd.0 != std::ptr::null_mut() {
            let mut buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut buf);
            if len > 0 {
                title = String::from_utf16_lossy(&buf[..len as usize]);
            }
        }
        
        let count = WORKSPACE.lock().unwrap().len();
        let payload = format!("{}|{}", count, title);

        if let Ok(socket) = UdpSocket::bind("127.0.0.1:0") {
            let _ = socket.send_to(payload.as_bytes(), "127.0.0.1:8123");
        }
    }
}

extern "system" fn win_event_proc(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _time: u32,
) {
    if hwnd.0 == std::ptr::null_mut() { return; }

    if event == EVENT_SYSTEM_FOREGROUND || event == EVENT_SYSTEM_DESKTOPSWITCH {
        broadcast_telemetry();
        update_layout();
    }

    let handle_val = hwnd.0 as isize;

    if event == EVENT_OBJECT_DESTROY || event == EVENT_OBJECT_HIDE {
        // Did OrbitBar get hidden? (Win+D)
        let title = unsafe { get_window_title(hwnd) };
        if title == "OrbitBar" {
            unsafe { let _ = ShowWindow(hwnd, SW_RESTORE); }
            return;
        }

        let mut ws = WORKSPACE.lock().unwrap();
        if let Some(pos) = ws.iter().position(|&x| x == handle_val) {
            ws.remove(pos);
            println!("[-] Removed Window");
            drop(ws);
            update_layout();
        }
        return;
    }

    if !is_manageable(hwnd) { return; }

    let title = unsafe { get_window_title(hwnd) };
    if title.is_empty() { return; }

    match event {
        EVENT_OBJECT_CREATE | EVENT_OBJECT_SHOW => {
            let mut ws = WORKSPACE.lock().unwrap();
            if !ws.contains(&handle_val) {
                ws.push(handle_val);
                println!("[+] Tiling: {}", title);
                drop(ws);
                update_layout();
            }
        },
        _ => {}
    }
}

extern "system" fn enum_windows_proc(hwnd: HWND, _lparam: LPARAM) -> windows::core::BOOL {
    if is_manageable(hwnd) {
        let title = unsafe { get_window_title(hwnd) };
        if !title.is_empty() {
            let mut ws = WORKSPACE.lock().unwrap();
            ws.push(hwnd.0 as isize);
            println!("[*] Found Existing: {}", title);
        }
    }
    true.into()
}

fn main() {
    println!("Orbit Core initialized.");
    println!("Listening for Window Events (Zero CPU usage)...");

    load_config();
    watch_config();
    register_hotkeys();

    unsafe {
        let _ = EnumWindows(Some(enum_windows_proc), LPARAM(0));
        update_layout();

        let hook_focus = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_DESKTOPSWITCH,
            None, Some(win_event_proc), 0, 0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );

        let hook_lifecycle = SetWinEventHook(
            EVENT_OBJECT_CREATE, EVENT_OBJECT_SHOW,
            None, Some(win_event_proc), 0, 0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );

        let mut msg: MSG = Default::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            if msg.message == WM_HOTKEY {
                match msg.wParam.0 as i32 {
                    HOTKEY_QUIT => {
                        println!("Exiting Orbit...");
                        PostQuitMessage(0);
                    },
                    HOTKEY_TERM => {
                        println!("Launching Terminal...");
                        let _ = Command::new("powershell.exe").creation_flags(0x00000010).spawn();
                    },
                    HOTKEY_CLOSE => {
                        let active = GetForegroundWindow();
                        if !active.0.is_null() {
                            let _ = SendMessageW(active, WM_CLOSE, Some(WPARAM(0)), Some(LPARAM(0)));
                        }
                    },
                    _ => {}
                }
            }
            DispatchMessageW(&msg);
        }

        let _ = UnhookWinEvent(hook_focus);
        let _ = UnhookWinEvent(hook_lifecycle);
    }
}
