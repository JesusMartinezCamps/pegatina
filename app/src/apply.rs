use crate::state::{lock, settings_json, AppState};
use crate::{panel, tray};
use pegatina_core::Settings;
use tauri::{AppHandle, Emitter, Manager, Theme};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

const THEMES: [&str; 3] = ["system", "light", "dark"];

fn parse_hotkey(text: &str) -> Result<Shortcut, String> {
    let shortcut: Shortcut = text
        .trim()
        .parse()
        .map_err(|_| "Ese atajo no es válido".to_string())?;
    if shortcut.mods.is_empty() {
        return Err("El atajo necesita Ctrl, Alt, Shift o Win además de la tecla".into());
    }
    Ok(shortcut)
}

/// Registra `new` y suelta `old`; si `new` falla, `old` sigue activo. Devuelve si hubo cambio.
pub fn change_hotkey(app: &AppHandle, old: &str, new: &str) -> Result<bool, String> {
    let new_shortcut = parse_hotkey(new)?;
    let old_shortcut = parse_hotkey(old).ok();
    if old_shortcut == Some(new_shortcut) {
        return Ok(false);
    }
    app.global_shortcut()
        .register(new_shortcut)
        .map_err(|_| "Ese atajo ya lo usa otra aplicación".to_string())?;
    if let Some(shortcut) = old_shortcut {
        let _ = app.global_shortcut().unregister(shortcut);
    }
    Ok(true)
}

pub fn set_autostart(app: &AppHandle, on: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    if manager.is_enabled().unwrap_or(false) == on {
        return Ok(());
    }
    let done = if on {
        manager.enable()
    } else {
        manager.disable()
    };
    done.map_err(|e| format!("No se pudo cambiar el arranque con Windows: {e}"))
}

pub fn apply_theme(app: &AppHandle, name: &str) {
    let theme = match name {
        "light" => Some(Theme::Light),
        "dark" => Some(Theme::Dark),
        _ => None,
    };
    for label in ["panel", "main"] {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.set_theme(theme);
        }
    }
}

pub fn emit_settings_changed(app: &AppHandle) {
    let value = settings_json(&app.state::<AppState>());
    let _ = app.emit("settings-changed", value);
}

pub fn apply(app: &AppHandle, mut new: Settings) -> Result<(), String> {
    let state = app.state::<AppState>();
    let old = lock(&state.settings).clone();
    new.hotkey = new.hotkey.trim().to_string();
    // Las carpetas solo cambian con `move_storage`, que también mueve los datos.
    new.text_dir = old.text_dir.clone();
    new.media_dir = old.media_dir.clone();
    if !THEMES.contains(&new.theme.as_str()) {
        new.theme = old.theme.clone();
    }

    let hotkey_changed = change_hotkey(app, &old.hotkey, &new.hotkey)?;
    let rest = set_autostart(app, new.autostart)
        .and_then(|()| new.save(&state.config_path).map_err(|e| format!("{e:#}")));
    if let Err(e) = rest {
        if hotkey_changed {
            let _ = change_hotkey(app, &new.hotkey, &old.hotkey);
        }
        let _ = set_autostart(app, old.autostart);
        return Err(e);
    }

    if new.history_limit < old.history_limit && lock(&state.store).prune(new.history_limit).is_ok()
    {
        let _ = app.emit("items-changed", ());
    }
    tray::sync_passwords(app, new.save_passwords);
    apply_theme(app, &new.theme);
    *lock(&state.settings) = new;
    emit_settings_changed(app);
    Ok(())
}

pub fn register_startup_hotkey(app: &AppHandle, hotkey: &str) {
    if let Err(e) = change_hotkey(app, "", hotkey) {
        eprintln!("No se pudo registrar el atajo «{hotkey}»: {e}");
        panel::show_main(app, "settings");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkeys_the_ui_can_produce_parse() {
        for text in ["Ctrl+Alt+Space", "Ctrl+Shift+V", "Super+V"] {
            assert!(parse_hotkey(text).is_ok(), "{text}");
        }
        for key in [
            "A",
            "1",
            "F5",
            "F24",
            "Space",
            "Enter",
            "Tab",
            "Backspace",
            "Delete",
            "Insert",
            "Home",
            "End",
            "PageUp",
            "PageDown",
            "ArrowUp",
            "ArrowDown",
            "ArrowLeft",
            "ArrowRight",
            "Minus",
            "Equal",
            "Comma",
            "Period",
            "Slash",
            "Backslash",
            "Semicolon",
            "Quote",
            "BracketLeft",
            "BracketRight",
            "Backquote",
            "Numpad0",
            "Numpad9",
        ] {
            assert!(
                parse_hotkey(&format!("Ctrl+Alt+Super+Shift+{key}")).is_ok(),
                "{key}"
            );
        }
    }

    #[test]
    fn hotkeys_without_modifier_or_with_junk_are_rejected() {
        assert!(parse_hotkey("Space").is_err());
        assert!(parse_hotkey("Ctrl+").is_err());
        assert!(parse_hotkey("Ctrl+Alt+Nada").is_err());
        assert!(parse_hotkey("").is_err());
    }
}
