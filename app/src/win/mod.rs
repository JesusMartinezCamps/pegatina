#[cfg(windows)]
mod clipboard;
#[cfg(windows)]
mod dpapi;
#[cfg(windows)]
mod focus;
#[cfg(not(windows))]
mod stub;

#[cfg(windows)]
pub use clipboard::{
    read_clipboard, registered_format, set_formats, start_listener, write_files, write_image,
    write_secret, write_text, Format,
};
#[cfg(windows)]
pub use dpapi::{protect, unprotect};
#[cfg(windows)]
pub use focus::{foreground_window, paste_into};
#[cfg(not(windows))]
pub use stub::*;

#[cfg_attr(not(windows), allow(dead_code))]
pub enum Content {
    Text(String),
    Image {
        png: Vec<u8>,
        width: u32,
        height: u32,
    },
    Files(Vec<String>),
    Secret(Vec<u8>),
}

pub struct Captured {
    pub content: Content,
    pub source_app: Option<String>,
}
