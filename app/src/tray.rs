use crate::state::{lock, AppState};
use crate::{apply, panel};
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

struct TrayItems {
    passwords: CheckMenuItem<Wry>,
}

pub fn build(app: &AppHandle, save_passwords: bool) -> tauri::Result<()> {
    let open_panel = MenuItem::with_id(app, "open_panel", "Abrir panel", true, None::<&str>)?;
    let library = MenuItem::with_id(app, "library", "Biblioteca", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Configuración", true, None::<&str>)?;
    let passwords = CheckMenuItem::with_id(
        app,
        "save_passwords",
        "Guardar contraseñas",
        true,
        save_passwords,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &open_panel,
            &library,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &passwords,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    TrayIconBuilder::with_id("pegatina")
        .icon(Image::from_bytes(include_bytes!("../icons/tray.png"))?)
        .tooltip("Pegatina")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open_panel" => panel::open_from_tray(app),
            "library" => panel::show_main(app, "library"),
            "settings" => panel::show_main(app, "settings"),
            "save_passwords" => toggle_passwords(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                panel::show_main(tray.app_handle(), "library");
            }
        })
        .build(app)?;

    app.manage(TrayItems { passwords });
    Ok(())
}

pub fn sync_passwords(app: &AppHandle, value: bool) {
    if let Some(items) = app.try_state::<TrayItems>() {
        let _ = items.passwords.set_checked(value);
    }
}

fn toggle_passwords(app: &AppHandle) {
    let Some(items) = app.try_state::<TrayItems>() else {
        return;
    };
    let checked = items.passwords.is_checked().unwrap_or(false);
    let mut settings = lock(&app.state::<AppState>().settings).clone();
    settings.save_passwords = checked;
    if let Err(e) = apply::apply(app, settings) {
        eprintln!("No se pudo cambiar «Guardar contraseñas»: {e}");
        let _ = items.passwords.set_checked(!checked);
    }
}
