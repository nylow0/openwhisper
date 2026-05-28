//! Types transcribed text into whatever window currently has keyboard focus.
//!
//! On Windows, uses `SendInput` with Unicode key events.
//! On Linux, uses `xdotool type` for X11 or clipboard paste as fallback.

/// Types `text` into the focused application.
pub fn type_text(text: &str) {
    platform::type_text(text);
}

#[cfg(windows)]
mod platform {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
        KEYEVENTF_UNICODE, VIRTUAL_KEY, VK_RETURN,
    };

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
}

#[cfg(target_os = "linux")]
mod platform {
    use std::process::Command;

    pub fn type_text(text: &str) {
        if text.is_empty() {
            return;
        }

        // Try xdotool first (works on X11).
        if try_xdotool(text) {
            return;
        }

        // Try wtype (works on Wayland).
        if try_wtype(text) {
            return;
        }

        // Fallback: clipboard paste via xclip + xdotool Ctrl+V.
        if try_clipboard_paste(text) {
            return;
        }

        log::warn!("No text injection method available. Install xdotool (X11) or wtype (Wayland).");
    }

    fn try_xdotool(text: &str) -> bool {
        match Command::new("xdotool")
            .args(["type", "--clearmodifiers", "--delay", "0", "--", text])
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    return true;
                }
                log::debug!("xdotool type failed: {}", String::from_utf8_lossy(&output.stderr));
                false
            }
            Err(e) => {
                log::debug!("xdotool not available: {e}");
                false
            }
        }
    }

    fn try_wtype(text: &str) -> bool {
        match Command::new("wtype").arg("--").arg(text).output() {
            Ok(output) => {
                if output.status.success() {
                    return true;
                }
                log::debug!("wtype failed: {}", String::from_utf8_lossy(&output.stderr));
                false
            }
            Err(e) => {
                log::debug!("wtype not available: {e}");
                false
            }
        }
    }

    fn try_clipboard_paste(text: &str) -> bool {
        let clipboard_ok = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(ref mut stdin) = child.stdin {
                    stdin.write_all(text.as_bytes())?;
                }
                child.wait()
            })
            .map(|status| status.success())
            .unwrap_or(false);

        if !clipboard_ok {
            log::debug!("xclip not available for clipboard paste");
            return false;
        }

        // Simulate Ctrl+V.
        Command::new("xdotool")
            .args(["key", "--clearmodifiers", "ctrl+v"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}
