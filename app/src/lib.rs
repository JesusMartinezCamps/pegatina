mod apply;
mod capture;
mod commands;
pub mod formats;
mod panel;
pub mod selftest;
mod state;
mod tray;
pub mod win;

use pegatina_core::{Settings, Store};
use state::{default_data_dir, lock, resolve_dirs, AppState};
use std::sync::atomic::{AtomicIsize, AtomicU64};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::ShortcutState;

const AUTOSTART_FLAG: &str = "--autostart";

fn on_clipboard_change(app: &AppHandle) {
    let state = app.state::<AppState>();
    let (save_passwords, limit) = {
        let settings = lock(&state.settings);
        (settings.save_passwords, settings.history_limit)
    };
    let captured = capture::capture_once(&state.store, save_passwords, limit);
    match captured {
        Ok(Some(_)) => {
            let _ = app.emit("items-changed", ());
        }
        Ok(None) => {}
        Err(e) => eprintln!("No se pudo guardar lo copiado: {e:#}"),
    }
}

fn open_store(settings: &Settings, default_dir: &std::path::Path) -> anyhow::Result<Store> {
    let (text_dir, media_dir) = resolve_dirs(settings, default_dir);
    Store::open(&text_dir, &media_dir).or_else(|e| {
        eprintln!(
            "No se pudo abrir {}: {e:#}; se usa la carpeta por defecto",
            text_dir.display()
        );
        let (text_dir, media_dir) = resolve_dirs(&Settings::default(), default_dir);
        Store::open(&text_dir, &media_dir)
    })
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();
    let default_dir = default_data_dir(&handle)?;
    let config_path = default_dir.join("config.json");
    let settings = Settings::load(&config_path);
    let store = open_store(&settings, &default_dir)?;
    let _ = store.prune(settings.history_limit);

    let hotkey = settings.hotkey.clone();
    let theme = settings.theme.clone();
    let autostart = settings.autostart;
    let save_passwords = settings.save_passwords;
    app.manage(AppState {
        store: Mutex::new(store),
        settings: Mutex::new(settings),
        config_path,
        previous_window: AtomicIsize::new(0),
        panel_shown_at: AtomicU64::new(0),
    });

    tray::build(&handle, save_passwords)?;
    apply::apply_theme(&handle, &theme);
    if let Err(e) = apply::set_autostart(&handle, autostart) {
        eprintln!("{e}");
    }
    apply::register_startup_hotkey(&handle, &hotkey);

    let listener_handle = handle.clone();
    if let Err(e) = win::start_listener(move || on_clipboard_change(&listener_handle)) {
        eprintln!("{e:#}");
    }

    if !std::env::args().any(|a| a == AUTOSTART_FLAG) {
        panel::show_main(&handle, "library");
    }
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            panel::show_main(app, "library");
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![AUTOSTART_FLAG]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        panel::toggle(app);
                    }
                })
                .build(),
        )
        .setup(setup)
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            WindowEvent::Focused(false) if window.label() == "panel" => {
                panel::hide_if_blurred(window.app_handle());
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_items,
            commands::copy_item,
            commands::paste_item,
            commands::reveal_secret,
            commands::image_data,
            commands::set_favorite,
            commands::set_space,
            commands::delete_item,
            commands::list_spaces,
            commands::create_space,
            commands::rename_space,
            commands::delete_space,
            commands::get_settings,
            commands::set_settings,
            commands::pick_folder,
            commands::move_storage,
            commands::clear_history,
            commands::open_main,
            commands::hide_panel,
        ])
        .run(tauri::generate_context!())
        .expect("Pegatina no pudo arrancar");
}
