use super::{dpapi, Captured, Content};
use crate::formats::{self, CF_DIB, CF_DIBV5, CF_HDROP, CF_UNICODETEXT};
use anyhow::{anyhow, bail, Result};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};
use std::sync::{mpsc, Mutex, MutexGuard, OnceLock};
use std::thread::sleep;
use std::time::Duration;
use windows::core::{w, Error, PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    CloseHandle, GlobalFree, HANDLE, HGLOBAL, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM,
};
use windows::Win32::System::DataExchange::{
    AddClipboardFormatListener, CloseClipboard, EmptyClipboard, GetClipboardData,
    GetClipboardOwner, GetClipboardSequenceNumber, IsClipboardFormatAvailable, OpenClipboard,
    RegisterClipboardFormatW, RemoveClipboardFormatListener, SetClipboardData,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetWindowThreadProcessId,
    RegisterClassW, TranslateMessage, HWND_MESSAGE, MSG, WINDOW_EX_STYLE, WINDOW_STYLE,
    WM_CLIPBOARDUPDATE, WNDCLASSW,
};

const EXCLUDE_FORMAT: &str = "ExcludeClipboardContentFromMonitorProcessing";
const HISTORY_FORMAT: &str = "CanIncludeInClipboardHistory";

pub struct Format {
    pub id: u32,
    pub data: Vec<u8>,
}

type Callback = Box<dyn Fn() + Send>;

static CALLBACK: Mutex<Option<Callback>> = Mutex::new(None);
// Rango de secuencias del portapapeles que escribió Pegatina; el listener las ignora.
static OWN_WRITES: Mutex<(u32, u32)> = Mutex::new((0, 0));
static LAST_SEQ: AtomicU32 = AtomicU32::new(0);
// Dueña del portapapeles cuando escribimos (SetClipboardData lo exige). Vive en un hilo que
// solo bombea mensajes: otra app que vacía el portapapeles le envía WM_DESTROYCLIPBOARD y
// el listener puede estar esperando un cerrojo mientras tanto.
static OWNER: AtomicIsize = AtomicIsize::new(0);

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn registered_format(name: &str) -> u32 {
    let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    unsafe { RegisterClipboardFormatW(PCWSTR(wide.as_ptr())) }
}

fn png_format() -> u32 {
    static ID: OnceLock<u32> = OnceLock::new();
    *ID.get_or_init(|| registered_format("PNG"))
}

struct Open;

impl Open {
    fn acquire(owner: Option<HWND>) -> Result<Open> {
        for _ in 0..20 {
            if unsafe { OpenClipboard(owner) }.is_ok() {
                return Ok(Open);
            }
            sleep(Duration::from_millis(25));
        }
        bail!("El portapapeles está ocupado por otra aplicación")
    }
}

impl Drop for Open {
    fn drop(&mut self) {
        let _ = unsafe { CloseClipboard() };
    }
}

fn available(format: u32) -> bool {
    unsafe { IsClipboardFormatAvailable(format) }.is_ok()
}

fn get_bytes(format: u32) -> Option<Vec<u8>> {
    unsafe {
        let handle = GetClipboardData(format).ok()?;
        let global = HGLOBAL(handle.0);
        let size = GlobalSize(global);
        let ptr = GlobalLock(global);
        if ptr.is_null() {
            return None;
        }
        let bytes = std::slice::from_raw_parts(ptr as *const u8, size).to_vec();
        let _ = GlobalUnlock(global);
        Some(bytes)
    }
}

fn get_text() -> Option<String> {
    get_bytes(CF_UNICODETEXT).map(|b| formats::utf16_text(&b))
}

fn history_blocked() -> bool {
    let format = registered_format(HISTORY_FORMAT);
    available(format)
        && get_bytes(format).is_some_and(|b| b.len() >= 4 && b[..4].iter().all(|&x| x == 0))
}

fn get_image() -> Option<Content> {
    let png = png_format();
    if available(png) {
        if let Some(bytes) = get_bytes(png) {
            if let Ok((width, height)) = formats::png_size(&bytes) {
                return Some(Content::Image {
                    png: bytes,
                    width,
                    height,
                });
            }
        }
    }
    [CF_DIBV5, CF_DIB].into_iter().find_map(|format| {
        let dib = if available(format) {
            get_bytes(format)?
        } else {
            return None;
        };
        let (png, width, height) = formats::dib_to_png(&dib).ok()?;
        Some(Content::Image { png, width, height })
    })
}

fn owner_app() -> Option<String> {
    unsafe {
        let owner = GetClipboardOwner().ok()?;
        let mut pid = 0u32;
        GetWindowThreadProcessId(owner, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 520];
        let mut len = buf.len() as u32;
        let queried = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(process);
        queried.ok()?;
        formats::app_name_from_exe(&String::from_utf16_lossy(&buf[..len as usize]))
    }
}

fn read_open(save_passwords: bool) -> Result<Option<Content>> {
    if available(registered_format(EXCLUDE_FORMAT)) || history_blocked() {
        if !save_passwords {
            return Ok(None);
        }
        return match get_text() {
            Some(text) if !text.is_empty() => {
                Ok(Some(Content::Secret(dpapi::protect(text.as_bytes())?)))
            }
            _ => Ok(None),
        };
    }
    if available(CF_HDROP) {
        let files = get_bytes(CF_HDROP)
            .map(|data| formats::parse_dropfiles(&data))
            .unwrap_or_default();
        if !files.is_empty() {
            return Ok(Some(Content::Files(files)));
        }
    }
    // Excel, Word y similares ponen texto e imagen a la vez: el texto es lo copiado.
    if let Some(text) = get_text().filter(|t| !t.trim().is_empty()) {
        return Ok(Some(Content::Text(text)));
    }
    Ok(get_image())
}

pub fn read_clipboard(save_passwords: bool) -> Result<Option<Captured>> {
    let source_app = owner_app();
    let _open = Open::acquire(None)?;
    Ok(read_open(save_passwords)?.map(|content| Captured {
        content,
        source_app,
    }))
}

fn set_bytes(format: u32, data: &[u8]) -> Result<()> {
    unsafe {
        let global = GlobalAlloc(GMEM_MOVEABLE, data.len().max(1))?;
        let ptr = GlobalLock(global);
        if ptr.is_null() {
            let _ = GlobalFree(Some(global));
            bail!("No se pudo reservar memoria para el portapapeles");
        }
        std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, data.len());
        let _ = GlobalUnlock(global);
        if let Err(e) = SetClipboardData(format, Some(HANDLE(global.0))) {
            let _ = GlobalFree(Some(global));
            return Err(anyhow!("No se pudo escribir en el portapapeles: {e}"));
        }
    }
    Ok(())
}

fn write(formats: &[Format]) -> Result<u32> {
    let owner = OWNER.load(Ordering::SeqCst);
    let _open = Open::acquire((owner != 0).then_some(HWND(owner as *mut _)))?;
    unsafe { EmptyClipboard() }.map_err(|e| anyhow!("No se pudo vaciar el portapapeles: {e}"))?;
    for f in formats {
        set_bytes(f.id, &f.data)?;
    }
    Ok(unsafe { GetClipboardSequenceNumber() })
}

/// Escribe como lo haría otra aplicación: Pegatina sí capturará el resultado.
pub fn set_formats(formats: &[Format]) -> Result<()> {
    write(formats).map(|_| ())
}

fn write_own(formats: &[Format]) -> Result<()> {
    let mut own = lock(&OWN_WRITES);
    let during = write(formats)?;
    *own = (during, unsafe { GetClipboardSequenceNumber() });
    Ok(())
}

pub fn write_text(text: &str) -> Result<()> {
    write_own(&[Format {
        id: CF_UNICODETEXT,
        data: formats::utf16_nul(text),
    }])
}

pub fn write_image(png: &[u8]) -> Result<()> {
    write_own(&[
        Format {
            id: CF_DIB,
            data: formats::png_to_dib(png)?,
        },
        Format {
            id: png_format(),
            data: png.to_vec(),
        },
    ])
}

pub fn write_files(paths: &[String]) -> Result<()> {
    write_own(&[Format {
        id: CF_HDROP,
        data: formats::build_dropfiles(paths),
    }])
}

pub fn write_secret(text: &str) -> Result<()> {
    write_own(&[
        Format {
            id: CF_UNICODETEXT,
            data: formats::utf16_nul(text),
        },
        Format {
            id: registered_format(EXCLUDE_FORMAT),
            data: vec![1, 0, 0, 0],
        },
    ])
}

fn on_update() {
    let seq = unsafe { GetClipboardSequenceNumber() };
    if seq != 0 {
        let (first, last) = *lock(&OWN_WRITES);
        if (first..=last).contains(&seq) || LAST_SEQ.swap(seq, Ordering::SeqCst) == seq {
            return;
        }
    }
    if let Some(callback) = lock(&CALLBACK).as_ref() {
        let _ = catch_unwind(AssertUnwindSafe(callback));
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_CLIPBOARDUPDATE {
        on_update();
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

type Ready = mpsc::Sender<std::result::Result<(), String>>;

fn create_message_window(listen: bool) -> windows::core::Result<HWND> {
    unsafe {
        let instance: HINSTANCE = GetModuleHandleW(PCWSTR::null())?.into();
        let class = if listen {
            w!("PegatinaClipboardListener")
        } else {
            w!("PegatinaClipboardOwner")
        };
        let wc = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: class,
            ..Default::default()
        };
        if RegisterClassW(&wc) == 0 {
            return Err(Error::from_thread());
        }
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            w!("Pegatina"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(instance),
            None,
        )?;
        if listen {
            AddClipboardFormatListener(hwnd)?;
        }
        Ok(hwnd)
    }
}

fn message_thread(listen: bool, ready: Ready) {
    let hwnd = match create_message_window(listen) {
        Ok(hwnd) => hwnd,
        Err(e) => {
            let _ = ready.send(Err(e.to_string()));
            return;
        }
    };
    if !listen {
        OWNER.store(hwnd.0 as isize, Ordering::SeqCst);
    }
    let _ = ready.send(Ok(()));
    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        if listen {
            let _ = RemoveClipboardFormatListener(hwnd);
        }
    }
}

fn spawn_message_thread(listen: bool, name: &str) -> Result<()> {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name(name.into())
        .spawn(move || message_thread(listen, tx))?;
    rx.recv()
        .map_err(|_| anyhow!("El hilo {name} se cerró al arrancar"))?
        .map_err(|e| anyhow!("No se pudo preparar el portapapeles: {e}"))
}

/// `on_change` se ejecuta en el hilo del listener cada vez que otra aplicación cambia el portapapeles.
pub fn start_listener<F: Fn() + Send + 'static>(on_change: F) -> Result<()> {
    *lock(&CALLBACK) = Some(Box::new(on_change));
    spawn_message_thread(false, "pegatina-propietario")?;
    spawn_message_thread(true, "pegatina-portapapeles")
}
