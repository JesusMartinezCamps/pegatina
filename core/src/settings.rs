use crate::model::Settings;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

impl Settings {
    pub fn load(path: &Path) -> Settings {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)
                .with_context(|| format!("no se pudo crear {}", dir.display()))?;
        }
        let json = serde_json::to_string_pretty(self)?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, json).with_context(|| format!("no se pudo escribir {}", tmp.display()))?;
        fs::rename(&tmp, path).with_context(|| format!("no se pudo guardar {}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let s = Settings::load(&dir.path().join("settings.json"));
        assert_eq!(s, Settings::default());
        assert!(s.autostart);
        assert_eq!(s.hotkey, "Ctrl+Alt+Space");
        assert_eq!(s.history_limit, 1000);
        assert!(!s.save_passwords);
        assert_eq!(s.theme, "system");
        assert_eq!(s.text_dir, None);
        assert_eq!(s.media_dir, None);
    }

    #[test]
    fn broken_json_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, "{ esto no es json").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
        fs::write(&path, r#"{"historyLimit":"muchos"}"#).unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
    }

    #[test]
    fn partial_json_fills_missing_with_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, r#"{"theme":"dark","textDirResolved":"C:\\x"}"#).unwrap();
        let s = Settings::load(&path);
        assert_eq!(s.theme, "dark");
        assert_eq!(s.history_limit, 1000);
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("settings.json");
        let s = Settings {
            autostart: false,
            hotkey: "Ctrl+Shift+V".into(),
            text_dir: Some("D:\\Datos".into()),
            media_dir: Some("D:\\Fotos".into()),
            history_limit: 5000,
            save_passwords: true,
            theme: "dark".into(),
        };
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);
        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("\"historyLimit\""));
        assert!(raw.contains("\"savePasswords\""));
    }
}
