use anyhow::{bail, Result};
use std::thread::sleep;
use std::time::Duration;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    VIRTUAL_KEY, VK_CONTROL, VK_V,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId, IsIconic, IsWindow, SetForegroundWindow,
    ShowWindow, SW_RESTORE,
};

pub fn foreground_window() -> isize {
    unsafe { GetForegroundWindow().0 as isize }
}

pub fn paste_into(window: isize) -> Result<()> {
    if window != 0 {
        let target = HWND(window as *mut _);
        if !unsafe { IsWindow(Some(target)) }.as_bool() {
            bail!("La ventana de destino ya no existe; el elemento queda copiado");
        }
        unsafe { focus(target) };
    }
    sleep(Duration::from_millis(60));
    send_ctrl_v()
}

// SetForegroundWindow solo funciona si somos el proceso con el foco o compartimos su cola de entrada.
unsafe fn focus(target: HWND) {
    if IsIconic(target).as_bool() {
        let _ = ShowWindow(target, SW_RESTORE);
    }
    let current = GetForegroundWindow();
    if current == target {
        return;
    }
    let this_thread = GetCurrentThreadId();
    let fg_thread = GetWindowThreadProcessId(current, None);
    let attached = fg_thread != 0
        && fg_thread != this_thread
        && AttachThreadInput(this_thread, fg_thread, true).as_bool();
    let _ = SetForegroundWindow(target);
    if attached {
        let _ = AttachThreadInput(this_thread, fg_thread, false);
    }
}

fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send_ctrl_v() -> Result<()> {
    let inputs = [
        key(VK_CONTROL, false),
        key(VK_V, false),
        key(VK_V, true),
        key(VK_CONTROL, true),
    ];
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        bail!("Windows no dejó enviar Ctrl+V; el elemento queda copiado");
    }
    Ok(())
}
