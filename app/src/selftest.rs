use std::fs;

/// `pegatina.exe --selftest <informe>`: comprueba el portapapeles real de Windows sin abrir la interfaz.
pub fn run(report_path: &str) -> i32 {
    let mut lines = Vec::new();
    let passed = match imp::execute(&mut lines) {
        Ok(passed) => passed,
        Err(e) => {
            lines.push(format!("ERROR  {e:#}"));
            false
        }
    };
    lines.push(if passed {
        "RESULTADO: todo correcto".into()
    } else {
        "RESULTADO: hay fallos".into()
    });
    let _ = fs::write(report_path, lines.join("\n") + "\n");
    if passed {
        0
    } else {
        1
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn execute(_lines: &mut Vec<String>) -> anyhow::Result<bool> {
        anyhow::bail!("La autocomprobación solo funciona en Windows")
    }
}

#[cfg(windows)]
mod imp {
    use crate::capture;
    use crate::formats::{self, CF_DIB, CF_HDROP, CF_UNICODETEXT};
    use crate::win::{self, Content, Format};
    use anyhow::{bail, ensure, Context, Result};
    use pegatina_core::{Item, Kind, ListFilter, Store};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{mpsc, Arc, Mutex};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    const WAIT: Duration = Duration::from_secs(5);
    const QUIET: Duration = Duration::from_millis(1200);
    const EXCLUDE_FORMAT: &str = "ExcludeClipboardContentFromMonitorProcessing";

    type Outcome = std::result::Result<Option<Item>, String>;
    type Case = fn(&Ctx) -> Result<String>;

    struct Ctx {
        store: Arc<Mutex<Store>>,
        save_passwords: Arc<AtomicBool>,
        captured: mpsc::Receiver<Outcome>,
        dir: std::path::PathBuf,
    }

    impl Ctx {
        fn drain(&self) {
            while self.captured.try_recv().is_ok() {}
        }

        fn wait_capture(&self) -> Result<Option<Item>> {
            match self.captured.recv_timeout(WAIT) {
                Ok(outcome) => outcome.map_err(anyhow::Error::msg),
                Err(_) => bail!("Pegatina no detectó el cambio del portapapeles"),
            }
        }

        fn expect_no_capture(&self) -> Result<()> {
            match self.captured.recv_timeout(QUIET) {
                Err(mpsc::RecvTimeoutError::Timeout) => Ok(()),
                Ok(_) => bail!("Pegatina recapturó una escritura suya"),
                Err(e) => bail!("{e}"),
            }
        }

        fn passwords(&self, on: bool) {
            self.save_passwords.store(on, Ordering::SeqCst);
        }

        fn stored(&self) -> Vec<Item> {
            self.store
                .lock()
                .unwrap()
                .list(&ListFilter::default())
                .unwrap_or_default()
        }
    }

    fn unique(prefix: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        format!("{prefix} pegatina-selftest {nanos}")
    }

    fn text_format(text: &str) -> Format {
        Format {
            id: CF_UNICODETEXT,
            data: formats::utf16_nul(text),
        }
    }

    fn exclude_mark() -> Format {
        Format {
            id: win::registered_format(EXCLUDE_FORMAT),
            data: vec![1, 0, 0, 0],
        }
    }

    fn png(w: u32, h: u32, rgb: [u8; 3]) -> Result<Vec<u8>> {
        let img = image::RgbaImage::from_pixel(w, h, image::Rgba([rgb[0], rgb[1], rgb[2], 255]));
        let mut out = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)?;
        Ok(out)
    }

    fn captured_text(c: &Ctx) -> Result<String> {
        let marker = unique("texto");
        c.drain();
        win::set_formats(&[text_format(&marker)])?;
        let item = c.wait_capture()?.context("no se guardó nada")?;
        ensure!(
            item.kind == Kind::Text,
            "el tipo es {:?}, no texto",
            item.kind
        );
        ensure!(
            item.text.as_deref() == Some(marker.as_str()),
            "el texto guardado no coincide"
        );
        Ok(format!(
            "id {}, origen {}",
            item.id,
            item.source_app.as_deref().unwrap_or("desconocido")
        ))
    }

    fn password_ignored(c: &Ctx) -> Result<String> {
        let marker = unique("clave apagada");
        c.passwords(false);
        c.drain();
        win::set_formats(&[text_format(&marker), exclude_mark()])?;
        ensure!(
            c.wait_capture()?.is_none(),
            "se guardó una contraseña con el interruptor apagado"
        );
        ensure!(
            !c.stored()
                .iter()
                .any(|i| i.text.as_deref() == Some(marker.as_str())),
            "la contraseña aparece en el historial"
        );
        Ok("ignorada".into())
    }

    fn password_saved_encrypted(c: &Ctx) -> Result<String> {
        let marker = unique("clave encendida");
        c.passwords(true);
        c.drain();
        win::set_formats(&[text_format(&marker), exclude_mark()])?;
        let item = c.wait_capture()?.context("no se guardó la contraseña")?;
        c.passwords(false);
        ensure!(
            item.kind == Kind::Secret,
            "el tipo es {:?}, no secreto",
            item.kind
        );
        ensure!(
            item.text.is_none(),
            "el texto de un secreto no debe ir en claro"
        );
        let blob = c.store.lock().unwrap().secret_blob(item.id)?;
        ensure!(
            !blob.windows(marker.len()).any(|w| w == marker.as_bytes()),
            "el dato guardado no está cifrado"
        );
        let plain = win::unprotect(&blob)?;
        ensure!(
            plain == marker.as_bytes(),
            "DPAPI no devolvió el texto original"
        );
        Ok("cifrado con DPAPI y recuperado".into())
    }

    fn image_with_png(c: &Ctx) -> Result<String> {
        let bytes = png(4, 4, [10, 120, 200])?;
        c.drain();
        win::set_formats(&[
            Format {
                id: CF_DIB,
                data: formats::png_to_dib(&bytes)?,
            },
            Format {
                id: win::registered_format("PNG"),
                data: bytes,
            },
        ])?;
        let item = c.wait_capture()?.context("no se guardó la imagen")?;
        ensure!(
            item.kind == Kind::Image,
            "el tipo es {:?}, no imagen",
            item.kind
        );
        Ok(item.preview)
    }

    fn image_dib_only(c: &Ctx) -> Result<String> {
        let bytes = png(5, 3, [200, 40, 90])?;
        c.drain();
        win::set_formats(&[Format {
            id: CF_DIB,
            data: formats::png_to_dib(&bytes)?,
        }])?;
        let item = c.wait_capture()?.context("no se guardó la imagen")?;
        ensure!(
            item.kind == Kind::Image,
            "el tipo es {:?}, no imagen",
            item.kind
        );
        ensure!(
            item.preview.starts_with("5×3"),
            "tamaño inesperado: {}",
            item.preview
        );
        Ok(item.preview)
    }

    fn files(c: &Ctx) -> Result<String> {
        let file = c
            .dir
            .join(format!("{}.txt", unique("fichero").replace(' ', "-")));
        std::fs::write(&file, "contenido")?;
        let path = file.to_string_lossy().into_owned();
        c.drain();
        win::set_formats(&[Format {
            id: CF_HDROP,
            data: formats::build_dropfiles(std::slice::from_ref(&path)),
        }])?;
        let item = c.wait_capture()?.context("no se guardaron los archivos")?;
        ensure!(
            item.kind == Kind::Files,
            "el tipo es {:?}, no archivos",
            item.kind
        );
        ensure!(item.files == vec![path], "las rutas guardadas no coinciden");
        Ok(item.preview)
    }

    fn own_writes_not_recaptured(c: &Ctx) -> Result<String> {
        let before = c.stored().len();
        let text = unique("propio");
        c.drain();
        win::write_text(&text)?;
        c.expect_no_capture()?;
        let read = win::read_clipboard(false)?.context("el portapapeles está vacío")?;
        ensure!(
            matches!(read.content, Content::Text(ref t) if *t == text),
            "no se leyó el texto escrito"
        );

        c.passwords(true);
        let secret = unique("secreto propio");
        win::write_secret(&secret)?;
        c.expect_no_capture()?;
        let read = win::read_clipboard(true)?.context("el portapapeles está vacío")?;
        c.passwords(false);
        match read.content {
            Content::Secret(blob) => ensure!(
                win::unprotect(&blob)? == secret.as_bytes(),
                "secreto distinto"
            ),
            _ => bail!("el secreto escrito no lleva la marca de exclusión"),
        }
        ensure!(
            c.stored().len() == before,
            "el historial creció con escrituras propias"
        );
        Ok("ignoradas".into())
    }

    fn writers_round_trip(c: &Ctx) -> Result<String> {
        let bytes = png(6, 2, [1, 2, 3])?;
        win::write_image(&bytes)?;
        match win::read_clipboard(false)?.map(|r| r.content) {
            Some(Content::Image {
                width: 6,
                height: 2,
                ..
            }) => {}
            _ => bail!("la imagen escrita no se lee igual"),
        }
        let file = c.dir.join("escrito.txt");
        std::fs::write(&file, "x")?;
        let path = file.to_string_lossy().into_owned();
        win::write_files(std::slice::from_ref(&path))?;
        match win::read_clipboard(false)?.map(|r| r.content) {
            Some(Content::Files(f)) if f == [path] => {}
            _ => bail!("los archivos escritos no se leen igual"),
        }
        Ok("imagen y archivos".into())
    }

    pub fn execute(lines: &mut Vec<String>) -> Result<bool> {
        let dir = std::env::temp_dir().join(format!("pegatina-selftest-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        let store = Arc::new(Mutex::new(Store::open(
            &dir.join("texto"),
            &dir.join("media"),
        )?));
        let save_passwords = Arc::new(AtomicBool::new(false));
        let (tx, captured) = mpsc::channel::<Outcome>();
        {
            let (store, save_passwords) = (store.clone(), save_passwords.clone());
            win::start_listener(move || {
                let outcome =
                    capture::capture_once(&store, save_passwords.load(Ordering::SeqCst), 1000)
                        .map_err(|e| format!("{e:#}"));
                let _ = tx.send(outcome);
            })?;
        }
        let ctx = Ctx {
            store,
            save_passwords,
            captured,
            dir,
        };

        let cases: [(&str, Case); 9] = [
            ("texto copiado por otra app", captured_text),
            (
                "contraseña ignorada con el interruptor apagado",
                password_ignored,
            ),
            (
                "contraseña guardada y cifrada con el interruptor encendido",
                password_saved_encrypted,
            ),
            ("imagen con formato PNG", image_with_png),
            ("imagen solo como DIB", image_dib_only),
            ("archivos copiados", files),
            (
                "escrituras propias no se recapturan",
                own_writes_not_recaptured,
            ),
            ("escritura de imagen y archivos", writers_round_trip),
            ("texto de nuevo tras todo lo anterior", captured_text),
        ];
        let mut all_ok = true;
        for (name, case) in cases {
            match case(&ctx) {
                Ok(detail) => lines.push(format!("OK     {name}: {detail}")),
                Err(e) => {
                    all_ok = false;
                    lines.push(format!("FALLO  {name}: {e:#}"));
                }
            }
        }
        Ok(all_ok)
    }
}
