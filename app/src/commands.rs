use crate::state::{lock, settings_json, AppState};
use crate::{apply, panel, win};
use anyhow::{anyhow, Result};
use base64::Engine;
use pegatina_core::{Item, Kind, ListFilter, Settings, Space};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

type CmdResult<T> = Result<T, String>;

fn msg(e: impl std::fmt::Display) -> String {
    format!("{e:#}")
}

fn items_changed(app: &AppHandle) {
    let _ = app.emit("items-changed", ());
}

fn write_to_clipboard(state: &AppState, id: i64) -> Result<()> {
    let (item, image, secret) = {
        let store = lock(&state.store);
        let item = store.get(id)?;
        let image = if item.kind == Kind::Image {
            Some(fs::read(store.image_path(id)?)?)
        } else {
            None
        };
        let secret = if item.kind == Kind::Secret {
            Some(store.secret_blob(id)?)
        } else {
            None
        };
        (item, image, secret)
    };
    match item.kind {
        Kind::Text | Kind::Link => win::write_text(
            &item
                .text
                .ok_or_else(|| anyhow!("El elemento no tiene texto"))?,
        )?,
        Kind::Image => win::write_image(&image.unwrap_or_default())?,
        Kind::Files => win::write_files(&item.files)?,
        Kind::Secret => {
            let plain = win::unprotect(&secret.unwrap_or_default())?;
            win::write_secret(&String::from_utf8_lossy(&plain))?
        }
    }
    lock(&state.store).touch(id)
}

#[tauri::command(async)]
pub fn list_items(state: State<'_, AppState>, filter: ListFilter) -> CmdResult<Vec<Item>> {
    lock(&state.store).list(&filter).map_err(msg)
}

#[tauri::command(async)]
pub fn copy_item(app: AppHandle, state: State<'_, AppState>, id: i64) -> CmdResult<()> {
    write_to_clipboard(&state, id).map_err(msg)?;
    items_changed(&app);
    Ok(())
}

#[tauri::command(async)]
pub fn paste_item(app: AppHandle, state: State<'_, AppState>, id: i64) -> CmdResult<()> {
    write_to_clipboard(&state, id).map_err(msg)?;
    items_changed(&app);
    panel::hide(&app);
    let target = state.previous_window.swap(0, Ordering::SeqCst);
    if target == 0 {
        return Ok(());
    }
    win::paste_into(target).map_err(msg)
}

#[tauri::command(async)]
pub fn reveal_secret(state: State<'_, AppState>, id: i64) -> CmdResult<String> {
    let blob = lock(&state.store).secret_blob(id).map_err(msg)?;
    let plain = win::unprotect(&blob).map_err(msg)?;
    Ok(String::from_utf8_lossy(&plain).into_owned())
}

#[tauri::command(async)]
pub fn image_data(state: State<'_, AppState>, id: i64) -> CmdResult<String> {
    let path = lock(&state.store).image_path(id).map_err(msg)?;
    let bytes = fs::read(&path).map_err(|e| format!("No se pudo leer la imagen: {e}"))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:image/png;base64,{encoded}"))
}

#[tauri::command(async)]
pub fn set_favorite(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    value: bool,
) -> CmdResult<()> {
    lock(&state.store).set_favorite(id, value).map_err(msg)?;
    items_changed(&app);
    Ok(())
}

#[tauri::command(async)]
pub fn set_space(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    space_id: Option<i64>,
) -> CmdResult<()> {
    lock(&state.store).set_space(id, space_id).map_err(msg)?;
    items_changed(&app);
    Ok(())
}

#[tauri::command(async)]
pub fn delete_item(app: AppHandle, state: State<'_, AppState>, id: i64) -> CmdResult<()> {
    lock(&state.store).delete(id).map_err(msg)?;
    items_changed(&app);
    Ok(())
}

#[tauri::command(async)]
pub fn list_spaces(state: State<'_, AppState>) -> CmdResult<Vec<Space>> {
    lock(&state.store).spaces().map_err(msg)
}

#[tauri::command(async)]
pub fn create_space(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    color: String,
) -> CmdResult<Space> {
    let space = lock(&state.store)
        .create_space(&name, &color)
        .map_err(msg)?;
    items_changed(&app);
    Ok(space)
}

#[tauri::command(async)]
pub fn rename_space(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    name: String,
) -> CmdResult<()> {
    lock(&state.store).rename_space(id, &name).map_err(msg)?;
    items_changed(&app);
    Ok(())
}

#[tauri::command(async)]
pub fn delete_space(app: AppHandle, state: State<'_, AppState>, id: i64) -> CmdResult<()> {
    lock(&state.store).delete_space(id).map_err(msg)?;
    items_changed(&app);
    Ok(())
}

#[tauri::command(async)]
pub fn get_settings(state: State<'_, AppState>) -> Value {
    settings_json(&state)
}

#[tauri::command(async)]
pub fn set_settings(app: AppHandle, settings: Settings) -> CmdResult<()> {
    apply::apply(&app, settings)
}

#[tauri::command(async)]
pub fn pick_folder(app: AppHandle) -> CmdResult<Option<String>> {
    let mut dialog = app.dialog().file();
    if let Some(main) = app.get_webview_window("main") {
        dialog = dialog.set_parent(&main);
    }
    Ok(dialog
        .blocking_pick_folder()
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned()))
}

#[tauri::command(async)]
pub fn move_storage(
    app: AppHandle,
    state: State<'_, AppState>,
    which: String,
    path: String,
) -> CmdResult<()> {
    let path = path.trim().to_string();
    if path.is_empty() {
        return Err("Elige una carpeta".into());
    }
    let target = PathBuf::from(&path);
    let mut settings = lock(&state.settings).clone();
    {
        let mut store = lock(&state.store);
        match which.as_str() {
            "text" => {
                store.move_text_dir(&target).map_err(msg)?;
                settings.text_dir = Some(path);
                if settings.media_dir.is_none() {
                    store
                        .move_media_dir(&target.join("multimedia"))
                        .map_err(msg)?;
                }
            }
            "media" => {
                store.move_media_dir(&target).map_err(msg)?;
                settings.media_dir = Some(path);
            }
            _ => return Err("Carpeta desconocida".into()),
        }
    }
    settings.save(&state.config_path).map_err(msg)?;
    *lock(&state.settings) = settings;
    apply::emit_settings_changed(&app);
    items_changed(&app);
    Ok(())
}

#[tauri::command(async)]
pub fn clear_history(app: AppHandle, state: State<'_, AppState>) -> CmdResult<()> {
    lock(&state.store).clear_history().map_err(msg)?;
    items_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn open_main(app: AppHandle, section: String) {
    panel::show_main(&app, &section);
}

#[tauri::command]
pub fn hide_panel(app: AppHandle) {
    panel::hide(&app);
}
