//! Global hold-to-dictate hotkey: Ctrl + Super (Win on Windows).
//!
//! On Windows, uses a low-level keyboard hook (`WH_KEYBOARD_LL`) for both
//! key-down and key-up with Win key swallowing.
//!
//! On Linux, uses evdev to read keyboard events from input devices, detecting
//! Ctrl + Super hold-to-talk gestures.

use tokio::sync::mpsc;

/// A transition of the hold-to-dictate combo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyEvent {
    /// Ctrl + Super became held together.
    Pressed,
    /// One of the two keys was released.
    Released,
}

pub fn spawn_listener() -> mpsc::Receiver<HotkeyEvent> {
    platform::spawn_listener()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Windows implementation
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(windows)]
mod platform {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::OnceLock;

    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
        UnhookWindowsHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN,
        WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    };

    const VK_LCONTROL: u32 = 0xA2;
    const VK_RCONTROL: u32 = 0xA3;
    const VK_CONTROL: u32 = 0x11;
    const VK_LWIN: u32 = 0x5B;
    const VK_RWIN: u32 = 0x5C;
    const KEY_DOWN_MASK: u16 = 0x8000;

    static HOTKEY_TX: OnceLock<mpsc::Sender<HotkeyEvent>> = OnceLock::new();

    static CTRL_DOWN: AtomicBool = AtomicBool::new(false);
    static WIN_DOWN: AtomicBool = AtomicBool::new(false);
    static COMBO_ACTIVE: AtomicBool = AtomicBool::new(false);
    static WIN_SUPPRESSED: AtomicBool = AtomicBool::new(false);
    static WIN_DOWN_DELIVERED: AtomicBool = AtomicBool::new(false);

    fn is_ctrl(vk: u32) -> bool {
        vk == VK_LCONTROL || vk == VK_RCONTROL || vk == VK_CONTROL
    }

    fn is_win(vk: u32) -> bool {
        vk == VK_LWIN || vk == VK_RWIN
    }

    fn async_key_down(vk: u32) -> bool {
        unsafe { (GetAsyncKeyState(vk as i32) as u16 & KEY_DOWN_MASK) != 0 }
    }

    fn key_down_after_event(vk_matches: bool, is_down: bool, is_up: bool, async_down: bool) -> bool {
        if vk_matches && is_down {
            true
        } else if vk_matches && is_up {
            false
        } else {
            async_down
        }
    }

    fn ctrl_down_after_event(vk: u32, is_down: bool, is_up: bool) -> bool {
        key_down_after_event(
            is_ctrl(vk),
            is_down,
            is_up,
            async_key_down(VK_LCONTROL) || async_key_down(VK_RCONTROL) || async_key_down(VK_CONTROL),
        )
    }

    fn win_down_after_event(vk: u32, is_down: bool, is_up: bool) -> bool {
        key_down_after_event(
            is_win(vk),
            is_down,
            is_up,
            async_key_down(VK_LWIN) || async_key_down(VK_RWIN),
        )
    }

    fn emit(event: HotkeyEvent) {
        if let Some(tx) = HOTKEY_TX.get() {
            let _ = tx.try_send(event);
        }
    }

    unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code != HC_ACTION as i32 {
            return CallNextHookEx(HHOOK::default(), code, wparam, lparam);
        }

        let info = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let vk = info.vkCode;
        let message = wparam.0 as u32;
        let is_down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
        let is_up = message == WM_KEYUP || message == WM_SYSKEYUP;

        let mut swallow = false;

        if is_ctrl(vk) || is_win(vk) {
            let ctrl_down = ctrl_down_after_event(vk, is_down, is_up);
            let win_down = win_down_after_event(vk, is_down, is_up);

            CTRL_DOWN.store(ctrl_down, Ordering::Relaxed);
            WIN_DOWN.store(win_down, Ordering::Relaxed);

            if is_win(vk) {
                if is_down {
                    if ctrl_down {
                        WIN_SUPPRESSED.store(true, Ordering::Relaxed);
                    }
                    if WIN_SUPPRESSED.load(Ordering::Relaxed) {
                        swallow = true;
                    } else {
                        WIN_DOWN_DELIVERED.store(true, Ordering::Relaxed);
                    }
                } else if is_up {
                    if WIN_SUPPRESSED.load(Ordering::Relaxed)
                        && !WIN_DOWN_DELIVERED.load(Ordering::Relaxed)
                    {
                        swallow = true;
                    }
                    WIN_SUPPRESSED.store(false, Ordering::Relaxed);
                    WIN_DOWN_DELIVERED.store(false, Ordering::Relaxed);
                }
            }
        }

        let combo = CTRL_DOWN.load(Ordering::Relaxed) && WIN_DOWN.load(Ordering::Relaxed);
        let was_active = COMBO_ACTIVE.load(Ordering::Relaxed);

        if combo && !was_active {
            COMBO_ACTIVE.store(true, Ordering::Relaxed);
            emit(HotkeyEvent::Pressed);
        } else if !combo && was_active {
            COMBO_ACTIVE.store(false, Ordering::Relaxed);
            emit(HotkeyEvent::Released);
        }

        if swallow {
            return LRESULT(1);
        }
        CallNextHookEx(HHOOK::default(), code, wparam, lparam)
    }

    pub fn spawn_listener() -> mpsc::Receiver<HotkeyEvent> {
        let (tx, rx) = mpsc::channel::<HotkeyEvent>(64);
        let _ = HOTKEY_TX.set(tx);

        std::thread::spawn(|| unsafe {
            let hinstance: HINSTANCE = match GetModuleHandleW(None) {
                Ok(module) => HINSTANCE(module.0),
                Err(e) => {
                    log::error!("GetModuleHandleW failed: {e}");
                    HINSTANCE::default()
                }
            };

            let hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), hinstance, 0) {
                Ok(hook) => hook,
                Err(e) => {
                    log::error!("Failed to install keyboard hook: {e}");
                    return;
                }
            };
            log::info!("Keyboard hook installed — hold Ctrl+Win to dictate");

            let mut msg = MSG::default();
            loop {
                let result = GetMessageW(&mut msg, HWND::default(), 0, 0);
                if result.0 <= 0 {
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let _ = UnhookWindowsHookEx(hook);
        });

        rx
    }

    #[cfg(test)]
    mod tests {
        use super::key_down_after_event;

        #[test]
        fn current_key_down_wins_over_stale_async_state() {
            assert!(key_down_after_event(true, true, false, false));
        }

        #[test]
        fn current_key_up_wins_over_stale_async_state() {
            assert!(!key_down_after_event(true, false, true, true));
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Linux implementation — evdev-based global keyboard monitoring
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use evdev::{Device, InputEventKind, Key};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    pub fn spawn_listener() -> mpsc::Receiver<HotkeyEvent> {
        let (tx, rx) = mpsc::channel::<HotkeyEvent>(64);

        std::thread::spawn(move || {
            if let Err(e) = run_evdev_listener(tx) {
                log::error!("Evdev hotkey listener failed: {e}");
            }
        });

        rx
    }

    fn find_keyboard_devices() -> Vec<Device> {
        let mut keyboards = Vec::new();
        let devices = match evdev::enumerate() {
            devices => devices,
        };

        for (_path, device) in devices {
            let keys = device.supported_keys();
            if let Some(keys) = keys {
                if keys.contains(Key::KEY_LEFTCTRL) && keys.contains(Key::KEY_LEFTMETA) {
                    keyboards.push(device);
                }
            }
        }

        keyboards
    }

    fn run_evdev_listener(tx: mpsc::Sender<HotkeyEvent>) -> anyhow::Result<()> {
        let keyboards = find_keyboard_devices();
        if keyboards.is_empty() {
            log::warn!("No keyboard devices found via evdev. Hotkey will not work. \
                        Ensure the user is in the 'input' group or run as root.");
            std::thread::park();
            return Ok(());
        }

        log::info!(
            "Monitoring {} keyboard device(s) for Ctrl+Super hotkey",
            keyboards.len()
        );

        let ctrl_down = Arc::new(AtomicBool::new(false));
        let super_down = Arc::new(AtomicBool::new(false));
        let combo_active = Arc::new(AtomicBool::new(false));

        let mut handles = Vec::new();

        for mut device in keyboards {
            let tx = tx.clone();
            let ctrl_down = ctrl_down.clone();
            let super_down = super_down.clone();
            let combo_active = combo_active.clone();

            let handle = std::thread::spawn(move || {
                loop {
                    match device.fetch_events() {
                        Ok(events) => {
                            for event in events {
                                if let InputEventKind::Key(key) = event.kind() {
                                    let value = event.value(); // 0=up, 1=down, 2=repeat
                                    let is_down = value == 1;
                                    let is_up = value == 0;

                                    match key {
                                        Key::KEY_LEFTCTRL | Key::KEY_RIGHTCTRL => {
                                            if is_down {
                                                ctrl_down.store(true, Ordering::Relaxed);
                                            } else if is_up {
                                                ctrl_down.store(false, Ordering::Relaxed);
                                            }
                                        }
                                        Key::KEY_LEFTMETA | Key::KEY_RIGHTMETA => {
                                            if is_down {
                                                super_down.store(true, Ordering::Relaxed);
                                            } else if is_up {
                                                super_down.store(false, Ordering::Relaxed);
                                            }
                                        }
                                        _ => continue,
                                    }

                                    let combo = ctrl_down.load(Ordering::Relaxed)
                                        && super_down.load(Ordering::Relaxed);
                                    let was_active = combo_active.load(Ordering::Relaxed);

                                    if combo && !was_active {
                                        combo_active.store(true, Ordering::Relaxed);
                                        let _ = tx.try_send(HotkeyEvent::Pressed);
                                    } else if !combo && was_active {
                                        combo_active.store(false, Ordering::Relaxed);
                                        let _ = tx.try_send(HotkeyEvent::Released);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("Error reading evdev events: {e}");
                            break;
                        }
                    }
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.join();
        }
        Ok(())
    }
}
