use anyhow::Result;
use pegatina_core::{Settings, Store};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicIsize, AtomicU64};
use std::sync::{Mutex, MutexGuard};
use tauri::{AppHandle, Manager};

pub struct AppState {
    pub store: Mutex<Store>,
    pub settings: Mutex<Settings>,
    pub config_path: PathBuf,
    pub previous_window: AtomicIsize,
    pub panel_shown_at: AtomicU64,
}

pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn default_data_dir(app: &AppHandle) -> Result<PathBuf> {
    let appdata = if cfg!(windows) {
        std::env::var_os("APPDATA")
    } else {
        None
    };
    match appdata {
        Some(dir) => Ok(PathBuf::from(dir).join("Pegatina")),
        None => Ok(app.path().app_data_dir()?),
    }
}

pub fn resolve_dirs(settings: &Settings, default_dir: &Path) -> (PathBuf, PathBuf) {
    let chosen = |dir: &Option<String>| {
        dir.as_deref()
            .map(str::trim)
            .filter(|d| !d.is_empty())
            .map(PathBuf::from)
    };
    let text = chosen(&settings.text_dir).unwrap_or_else(|| default_dir.to_path_buf());
    let media = chosen(&settings.media_dir).unwrap_or_else(|| text.join("multimedia"));
    (text, media)
}

pub fn settings_json(state: &AppState) -> Value {
    let settings = lock(&state.settings).clone();
    let store = lock(&state.store);
    let mut value = serde_json::to_value(&settings).unwrap_or_else(|_| json!({}));
    value["textDirResolved"] = json!(store.text_dir().to_string_lossy());
    value["mediaDirResolved"] = json!(store.media_dir().to_string_lossy());
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirs_default_to_app_folder_and_multimedia_subfolder() {
        let (text, media) = resolve_dirs(&Settings::default(), Path::new("/datos/Pegatina"));
        assert_eq!(text, Path::new("/datos/Pegatina"));
        assert_eq!(media, Path::new("/datos/Pegatina/multimedia"));
    }

    #[test]
    fn media_follows_a_custom_text_dir_unless_set() {
        let mut s = Settings {
            text_dir: Some("/otra".into()),
            ..Settings::default()
        };
        assert_eq!(
            resolve_dirs(&s, Path::new("/d")).1,
            Path::new("/otra/multimedia")
        );
        s.media_dir = Some("/fotos".into());
        assert_eq!(
            resolve_dirs(&s, Path::new("/d")),
            (PathBuf::from("/otra"), PathBuf::from("/fotos"))
        );
        s.text_dir = Some("  ".into());
        assert_eq!(resolve_dirs(&s, Path::new("/d")).0, Path::new("/d"));
    }
}
