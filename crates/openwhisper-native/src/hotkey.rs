//! Global hold-to-dictate hotkey: Ctrl + Win.
//!
//! Electron's `globalShortcut` only fires on key-down, so a push-to-talk
//! "hold" gesture cannot be detected there. A low-level keyboard hook
//! (`WH_KEYBOARD_LL`) gives us both key-down and key-up globally, and lets us
//! swallow the Win key while the combo is engaged so the Start menu never
//! opens.
//!
//! Swallowing is kept *balanced*: a Win key-up is suppressed only when every
//! key-down of that hold was suppressed too. Otherwise the OS would be left
//! believing the Win key is still held down.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use tokio::sync::mpsc;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
    UnhookWindowsHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN,
    WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

/// A transition of the hold-to-dictate combo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyEvent {
    /// Ctrl + Win became held together.
    Pressed,
    /// One of the two keys was released.
    Released,
}

// Virtual-key codes. A low-level hook reports the side-specific Ctrl codes.
const VK_LCONTROL: u32 = 0xA2;
const VK_RCONTROL: u32 = 0xA3;
const VK_CONTROL: u32 = 0x11;
const VK_LWIN: u32 = 0x5B;
const VK_RWIN: u32 = 0x5C;
const KEY_DOWN_MASK: u16 = 0x8000;

static HOTKEY_TX: OnceLock<mpsc::Sender<HotkeyEvent>> = OnceLock::new();

// Hook state. The callback only ever runs on the single hook thread, so
// `Relaxed` ordering is sufficient — the atomics are just for `static` access.
static CTRL_DOWN: AtomicBool = AtomicBool::new(false);
static WIN_DOWN: AtomicBool = AtomicBool::new(false);
static COMBO_ACTIVE: AtomicBool = AtomicBool::new(false);
// Whether the current Win key hold is being swallowed as part of the combo.
static WIN_SUPPRESSED: AtomicBool = AtomicBool::new(false);
// Whether any Win key-down of the current hold reached the OS.
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
    // Low-level hooks run before Windows updates async state for this event.
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
                // Win pressed while Ctrl is held → this is the hotkey combo.
                // Latch suppression so the whole hold is swallowed consistently,
                // even through key auto-repeat or Ctrl being released first.
                if ctrl_down {
                    WIN_SUPPRESSED.store(true, Ordering::Relaxed);
                }
                if WIN_SUPPRESSED.load(Ordering::Relaxed) {
                    swallow = true;
                } else {
                    WIN_DOWN_DELIVERED.store(true, Ordering::Relaxed);
                }
            } else if is_up {
                // Swallow the key-up only if every key-down was swallowed too, so
                // the OS's view of the Win key stays balanced (never stuck down).
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

/// Installs the global keyboard hook on a dedicated thread and returns a
/// receiver of hold-to-dictate transitions.
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

        // A low-level hook is dispatched on the installing thread while it
        // pumps messages, so this loop must run for the hook to fire.
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
