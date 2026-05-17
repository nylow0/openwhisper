//! Types transcribed text into whatever window currently has keyboard focus,
//! using `SendInput` with Unicode key events.

use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, VIRTUAL_KEY, VK_RETURN,
};

/// A single Unicode UTF-16 code unit, sent as a synthetic key event.
fn unicode_event(unit: u16, key_up: bool) -> INPUT {
    let flags = if key_up {
        KEYEVENTF_UNICODE | KEYEVENTF_KEYUP
    } else {
        KEYEVENTF_UNICODE
    };
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: unit,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// A virtual-key event (used for keys that have no Unicode form, e.g. Enter).
fn vk_event(vk: VIRTUAL_KEY, key_up: bool) -> INPUT {
    let flags = if key_up {
        KEYEVENTF_KEYUP
    } else {
        KEYBD_EVENT_FLAGS(0)
    };
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// Types `text` into the focused application.
pub fn type_text(text: &str) {
    let mut inputs: Vec<INPUT> = Vec::with_capacity(text.len() * 2);
    let mut buffer = [0u16; 2];

    for ch in text.chars() {
        match ch {
            '\r' => {}
            '\n' => {
                inputs.push(vk_event(VK_RETURN, false));
                inputs.push(vk_event(VK_RETURN, true));
            }
            _ => {
                for unit in ch.encode_utf16(&mut buffer) {
                    inputs.push(unicode_event(*unit, false));
                    inputs.push(unicode_event(*unit, true));
                }
            }
        }
    }

    if inputs.is_empty() {
        return;
    }

    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if (sent as usize) != inputs.len() {
        log::warn!("SendInput injected {sent}/{} key events", inputs.len());
    }
}
