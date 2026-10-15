use crate::win::{self, Content};
use anyhow::Result;
use pegatina_core::{Item, Store};
use std::sync::{Mutex, PoisonError};

/// Lee el portapapeles sin tener el Store bloqueado (la lectura puede esperar a otra aplicación).
pub fn capture_once(
    store: &Mutex<Store>,
    save_passwords: bool,
    history_limit: u32,
) -> Result<Option<Item>> {
    let Some(captured) = win::read_clipboard(save_passwords)? else {
        return Ok(None);
    };
    let source = captured.source_app.as_deref();
    let store = store.lock().unwrap_or_else(PoisonError::into_inner);
    let item = match captured.content {
        Content::Text(text) => store.add_text(&text, source)?,
        Content::Image { png, width, height } => store.add_image(&png, width, height, source)?,
        Content::Files(paths) => store.add_files(&paths, source)?,
        Content::Secret(cipher) => store.add_secret(&cipher, source)?,
    };
    store.prune(history_limit)?;
    Ok(Some(item))
}
