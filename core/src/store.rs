use crate::model::{Item, Kind, ListFilter, Space};
use anyhow::{anyhow, bail, Context, Result};
use rusqlite::functions::FunctionFlags;
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension, Params, Row};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DB_FILE: &str = "pegatina.db";
const COLORS: [&str; 8] = [
    "blue", "orange", "green", "purple", "pink", "teal", "red", "gray",
];
const SECRET_PREVIEW: &str = "••••••••";
const PREVIEW_CHARS: usize = 300;
const DEFAULT_LIMIT: u32 = 200;
const COLS: &str =
    "id, kind, preview, text, image_file, files, source_app, favorite, space_id, created_at, used_at";

// Cada entrada lleva de la versión N a la N+1 (PRAGMA user_version).
const MIGRATIONS: &[&str] = &["
    CREATE TABLE spaces (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        color TEXT NOT NULL
    );
    CREATE TABLE items (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        kind TEXT NOT NULL,
        preview TEXT NOT NULL,
        text TEXT,
        image_file TEXT,
        files TEXT,
        secret BLOB,
        dedupe_key TEXT,
        source_app TEXT,
        favorite INTEGER NOT NULL DEFAULT 0,
        space_id INTEGER REFERENCES spaces(id) ON DELETE SET NULL,
        created_at INTEGER NOT NULL,
        used_at INTEGER NOT NULL
    );
    CREATE INDEX idx_items_used_at ON items(used_at DESC);
    CREATE INDEX idx_items_favorite_used_at ON items(favorite, used_at DESC);
    CREATE INDEX idx_items_space_used_at ON items(space_id, used_at DESC);
    CREATE INDEX idx_items_image ON items(image_file) WHERE image_file IS NOT NULL;
    CREATE UNIQUE INDEX idx_items_dedupe ON items(dedupe_key) WHERE dedupe_key IS NOT NULL;
"];

pub struct Store {
    conn: Connection,
    text_dir: PathBuf,
    media_dir: PathBuf,
}

struct NewItem<'a> {
    kind: Kind,
    preview: String,
    text: Option<&'a str>,
    image_file: Option<String>,
    files: Option<String>,
    secret: Option<&'a [u8]>,
    key: Option<String>,
    source_app: Option<&'a str>,
}

impl<'a> NewItem<'a> {
    fn new(kind: Kind, preview: String, source_app: Option<&'a str>) -> Self {
        NewItem {
            kind,
            preview,
            text: None,
            image_file: None,
            files: None,
            secret: None,
            key: None,
            source_app,
        }
    }
}

impl Store {
    pub fn open(text_dir: &Path, media_dir: &Path) -> Result<Store> {
        for dir in [text_dir, media_dir] {
            fs::create_dir_all(dir)
                .with_context(|| format!("No se pudo crear la carpeta {}", dir.display()))?;
        }
        let conn = open_db(&text_dir.join(DB_FILE))?;
        Ok(Store {
            conn,
            text_dir: text_dir.to_path_buf(),
            media_dir: media_dir.to_path_buf(),
        })
    }

    pub fn text_dir(&self) -> &Path {
        &self.text_dir
    }

    pub fn media_dir(&self) -> &Path {
        &self.media_dir
    }

    pub fn image_path(&self, id: i64) -> Result<PathBuf> {
        let item = self.get(id)?;
        let file = item
            .image_file
            .ok_or_else(|| anyhow!("El elemento {id} no es una imagen"))?;
        Ok(self.media_dir.join(file))
    }

    pub fn add_text(&self, text: &str, source_app: Option<&str>) -> Result<Item> {
        if text.trim().is_empty() {
            bail!("El texto está vacío");
        }
        let kind = if is_link(text) {
            Kind::Link
        } else {
            Kind::Text
        };
        let mut n = NewItem::new(kind, text_preview(text), source_app);
        n.text = Some(text);
        n.key = Some(format!("t:{}", sha_hex(text.as_bytes())));
        self.add(n)
    }

    pub fn add_image(&self, png: &[u8], w: u32, h: u32, source_app: Option<&str>) -> Result<Item> {
        if png.is_empty() {
            bail!("La imagen está vacía");
        }
        let hash = sha_hex(png);
        let file = format!("{hash}.png");
        let path = self.media_dir.join(&file);
        if !path.exists() {
            fs::create_dir_all(&self.media_dir)?;
            fs::write(&path, png)
                .with_context(|| format!("No se pudo guardar la imagen en {}", path.display()))?;
        }
        let mut n = NewItem::new(Kind::Image, format!("{w}×{h}"), source_app);
        n.image_file = Some(file);
        n.key = Some(format!("i:{hash}"));
        self.add(n)
    }

    pub fn add_files(&self, paths: &[String], source_app: Option<&str>) -> Result<Item> {
        if paths.is_empty() {
            bail!("No hay archivos que guardar");
        }
        let joined = paths.join("\n");
        let preview = paths
            .iter()
            .map(|p| file_name(p))
            .collect::<Vec<_>>()
            .join(", ");
        let mut n = NewItem::new(Kind::Files, preview, source_app);
        n.key = Some(format!("f:{}", sha_hex(joined.as_bytes())));
        n.files = Some(joined);
        self.add(n)
    }

    // El texto cifrado de DPAPI es distinto cada vez: un secret nunca se deduplica.
    pub fn add_secret(&self, ciphertext: &[u8], source_app: Option<&str>) -> Result<Item> {
        if ciphertext.is_empty() {
            bail!("El secreto está vacío");
        }
        let mut n = NewItem::new(Kind::Secret, SECRET_PREVIEW.into(), source_app);
        n.secret = Some(ciphertext);
        self.add(n)
    }

    pub fn secret_blob(&self, id: i64) -> Result<Vec<u8>> {
        self.conn
            .query_row(
                "SELECT secret FROM items WHERE id = ?1 AND kind = 'secret'",
                [id],
                |r| r.get::<_, Option<Vec<u8>>>(0),
            )
            .optional()?
            .flatten()
            .ok_or_else(|| anyhow!("El elemento {id} no es un secreto"))
    }

    pub fn get(&self, id: i64) -> Result<Item> {
        self.conn
            .query_row(
                &format!("SELECT {COLS} FROM items WHERE id = ?1"),
                [id],
                row_to_item,
            )
            .optional()?
            .ok_or_else(|| anyhow!("El elemento {id} no existe"))
    }

    pub fn list(&self, f: &ListFilter) -> Result<Vec<Item>> {
        let mut sql = format!("SELECT {COLS} FROM items WHERE 1 = 1");
        let mut args: Vec<Value> = Vec::new();
        match f.view.as_str() {
            "recent" => {}
            "favorites" => sql.push_str(" AND favorite = 1"),
            "space" => match f.space_id {
                Some(id) => {
                    args.push(Value::Integer(id));
                    sql.push_str(&format!(" AND space_id = ?{}", args.len()));
                }
                None => return Ok(Vec::new()),
            },
            other => bail!("Vista desconocida: {other}"),
        }
        if let Some(kind) = f.kind {
            args.push(Value::Text(kind.as_str().into()));
            sql.push_str(&format!(" AND kind = ?{}", args.len()));
        }
        if let Some(q) = f.query.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
            args.push(Value::Text(q.to_lowercase()));
            let n = args.len();
            sql.push_str(&format!(
                " AND (instr(ulower(text), ?{n}) > 0 OR instr(ulower(preview), ?{n}) > 0 \
                 OR instr(ulower(files), ?{n}) > 0)"
            ));
        }
        args.push(Value::Integer(i64::from(f.limit.unwrap_or(DEFAULT_LIMIT))));
        sql.push_str(&format!(
            " ORDER BY used_at DESC, id DESC LIMIT ?{}",
            args.len()
        ));
        let mut st = self.conn.prepare(&sql)?;
        let rows = st.query_map(params_from_iter(args), row_to_item)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn touch(&self, id: i64) -> Result<()> {
        let ts = self.next_ts()?;
        let n = self.conn.execute(
            "UPDATE items SET used_at = ?1 WHERE id = ?2",
            params![ts, id],
        )?;
        expect_changed(n, id)
    }

    pub fn set_favorite(&self, id: i64, favorite: bool) -> Result<()> {
        let n = self.conn.execute(
            "UPDATE items SET favorite = ?1 WHERE id = ?2",
            params![favorite, id],
        )?;
        expect_changed(n, id)
    }

    pub fn set_space(&self, id: i64, space_id: Option<i64>) -> Result<()> {
        let n = self
            .conn
            .execute(
                "UPDATE items SET space_id = ?1 WHERE id = ?2",
                params![space_id, id],
            )
            .context("Ese espacio no existe")?;
        expect_changed(n, id)
    }

    pub fn delete(&self, id: i64) -> Result<()> {
        self.delete_where("id = ?1", [id])
    }

    pub fn clear_history(&self) -> Result<()> {
        self.delete_where("favorite = 0 AND space_id IS NULL", [])
    }

    // Los favoritos y lo que tiene espacio no cuentan para el límite.
    pub fn prune(&self, limit: u32) -> Result<()> {
        self.delete_where(
            "favorite = 0 AND space_id IS NULL AND id NOT IN (
                SELECT id FROM items WHERE favorite = 0 AND space_id IS NULL
                ORDER BY used_at DESC, id DESC LIMIT ?1)",
            [i64::from(limit)],
        )
    }

    pub fn spaces(&self) -> Result<Vec<Space>> {
        let mut st = self
            .conn
            .prepare("SELECT id, name, color FROM spaces ORDER BY id")?;
        let rows = st.query_map([], |r| {
            Ok(Space {
                id: r.get(0)?,
                name: r.get(1)?,
                color: r.get(2)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn create_space(&self, name: &str, color: &str) -> Result<Space> {
        let name = clean_space_name(name)?;
        if !COLORS.contains(&color) {
            bail!("Color de espacio desconocido: {color}");
        }
        self.conn.execute(
            "INSERT INTO spaces (name, color) VALUES (?1, ?2)",
            params![name, color],
        )?;
        Ok(Space {
            id: self.conn.last_insert_rowid(),
            name,
            color: color.into(),
        })
    }

    pub fn rename_space(&self, id: i64, name: &str) -> Result<()> {
        let name = clean_space_name(name)?;
        let n = self.conn.execute(
            "UPDATE spaces SET name = ?1 WHERE id = ?2",
            params![name, id],
        )?;
        if n == 0 {
            bail!("El espacio {id} no existe");
        }
        Ok(())
    }

    pub fn delete_space(&self, id: i64) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("UPDATE items SET space_id = NULL WHERE space_id = ?1", [id])?;
        tx.execute("DELETE FROM spaces WHERE id = ?1", [id])?;
        Ok(tx.commit()?)
    }

    pub fn move_text_dir(&mut self, new: &Path) -> Result<()> {
        fs::create_dir_all(new)
            .with_context(|| format!("No se pudo crear la carpeta {}", new.display()))?;
        if same_dir(&self.text_dir, new) {
            return Ok(());
        }
        let old_db = self.text_dir.join(DB_FILE);
        let new_db = new.join(DB_FILE);
        if new_db.exists() {
            bail!("Ya hay una base de datos de Pegatina en esa carpeta");
        }
        fs::copy(&old_db, &new_db).context("No se pudo copiar la base de datos")?;
        let reopened = open_db(&new_db)
            .and_then(|c| verify_copy(&self.conn, &c).map(|()| c))
            .inspect_err(|_| {
                let _ = fs::remove_file(&new_db);
            })?;
        self.conn = reopened;
        self.text_dir = new.to_path_buf();
        let _ = fs::remove_file(&old_db);
        let _ = fs::remove_file(old_db.with_extension("db-journal"));
        Ok(())
    }

    pub fn move_media_dir(&mut self, new: &Path) -> Result<()> {
        fs::create_dir_all(new)
            .with_context(|| format!("No se pudo crear la carpeta {}", new.display()))?;
        let old = self.media_dir.clone();
        if same_dir(&old, new) {
            return Ok(());
        }
        let names = png_names(&old);
        let mut created: Vec<PathBuf> = Vec::new();
        for name in &names {
            let (src, dst) = (old.join(name), new.join(name));
            let existed = dst.exists();
            let copied = fs::copy(&src, &dst)
                .map_err(anyhow::Error::from)
                .and_then(|n| {
                    if n == fs::metadata(&src)?.len() {
                        Ok(())
                    } else {
                        bail!("La copia de {name} salió incompleta")
                    }
                });
            if !existed {
                created.push(dst);
            }
            if let Err(e) = copied {
                for f in &created {
                    let _ = fs::remove_file(f);
                }
                return Err(e.context("No se pudieron mover las imágenes"));
            }
        }
        self.media_dir = new.to_path_buf();
        for name in &names {
            let _ = fs::remove_file(old.join(name));
        }
        let _ = fs::remove_dir(&old);
        Ok(())
    }

    fn add(&self, n: NewItem) -> Result<Item> {
        if let Some(key) = &n.key {
            let found = self
                .conn
                .query_row("SELECT id FROM items WHERE dedupe_key = ?1", [key], |r| {
                    r.get::<_, i64>(0)
                })
                .optional()?;
            if let Some(id) = found {
                self.touch(id)?;
                return self.get(id);
            }
        }
        let ts = self.next_ts()?;
        let source_app = n.source_app.filter(|s| !s.is_empty());
        self.conn.execute(
            "INSERT INTO items
                (kind, preview, text, image_file, files, secret, dedupe_key, source_app,
                 created_at, used_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![
                n.kind.as_str(),
                n.preview,
                n.text,
                n.image_file,
                n.files,
                n.secret,
                n.key,
                source_app,
                ts
            ],
        )?;
        self.get(self.conn.last_insert_rowid())
    }

    // Estrictamente creciente: el orden por used_at no tiene empates aunque dos copias caigan en el mismo ms.
    fn next_ts(&self) -> Result<i64> {
        let max: Option<i64> = self
            .conn
            .query_row("SELECT MAX(used_at) FROM items", [], |r| r.get(0))?;
        Ok(now_ms().max(max.unwrap_or(0) + 1))
    }

    fn delete_where(&self, cond: &str, params: impl Params) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        let rows: Vec<(i64, Option<String>)> = {
            let mut st = tx.prepare(&format!("SELECT id, image_file FROM items WHERE {cond}"))?;
            let mapped = st.query_map(params, |r| Ok((r.get(0)?, r.get(1)?)))?;
            mapped.collect::<rusqlite::Result<_>>()?
        };
        {
            let mut del = tx.prepare("DELETE FROM items WHERE id = ?1")?;
            for (id, _) in &rows {
                del.execute([id])?;
            }
        }
        tx.commit()?;
        for file in rows.iter().filter_map(|(_, f)| f.as_deref()) {
            self.drop_image_if_unused(file);
        }
        Ok(())
    }

    fn drop_image_if_unused(&self, file: &str) {
        let used = self
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM items WHERE image_file = ?1)",
                [file],
                |r| r.get::<_, bool>(0),
            )
            .unwrap_or(true);
        if !used {
            let _ = fs::remove_file(self.media_dir.join(file));
        }
    }
}

fn open_db(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)
        .with_context(|| format!("No se pudo abrir la base de datos {}", path.display()))?;
    conn.pragma_update(None, "foreign_keys", true)?;
    // LIKE solo ignora mayúsculas en ASCII; el buscador necesita Á = á.
    conn.create_scalar_function(
        "ulower",
        1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| Ok(ctx.get::<Option<String>>(0)?.map(|s| s.to_lowercase())),
    )?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<()> {
    let current: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if current > MIGRATIONS.len() as i64 {
        bail!("La base de datos es de una versión más nueva de Pegatina");
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", i as i64 + 1)?;
        tx.commit()?;
    }
    Ok(())
}

fn verify_copy(old: &Connection, new: &Connection) -> Result<()> {
    let integrity: String = new.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    if integrity != "ok" {
        bail!("La copia de la base de datos está dañada");
    }
    let count = |c: &Connection| -> Result<(i64, i64)> {
        Ok(c.query_row(
            "SELECT (SELECT COUNT(*) FROM items), (SELECT COUNT(*) FROM spaces)",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?)
    };
    if count(old)? != count(new)? {
        bail!("La copia de la base de datos salió incompleta");
    }
    Ok(())
}

fn row_to_item(r: &Row) -> rusqlite::Result<Item> {
    let kind: String = r.get(1)?;
    let files: Option<String> = r.get(5)?;
    Ok(Item {
        id: r.get(0)?,
        kind: Kind::parse(&kind).ok_or_else(|| {
            rusqlite::Error::InvalidColumnType(1, "kind".into(), rusqlite::types::Type::Text)
        })?,
        preview: r.get(2)?,
        text: r.get(3)?,
        image_file: r.get(4)?,
        files: files
            .map(|f| {
                f.split('\n')
                    .filter(|p| !p.is_empty())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default(),
        source_app: r.get(6)?,
        favorite: r.get(7)?,
        space_id: r.get(8)?,
        created_at: r.get(9)?,
        used_at: r.get(10)?,
    })
}

fn expect_changed(changed: usize, id: i64) -> Result<()> {
    if changed == 0 {
        bail!("El elemento {id} no existe");
    }
    Ok(())
}

fn clean_space_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        bail!("El espacio necesita un nombre");
    }
    Ok(name.into())
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn sha_hex(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn text_preview(text: &str) -> String {
    text.trim().chars().take(PREVIEW_CHARS).collect()
}

fn is_link(text: &str) -> bool {
    let t = text.trim();
    let rest = ["https://", "http://"].iter().find_map(|scheme| {
        t.get(..scheme.len())
            .filter(|head| head.eq_ignore_ascii_case(scheme))
            .map(|_| &t[scheme.len()..])
    });
    let Some(rest) = rest else { return false };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    !host.is_empty() && !rest.chars().any(|c| c.is_whitespace() || c.is_control())
}

// Las rutas llegan de Windows (con `\`) aunque los tests corran en Linux.
fn file_name(path: &str) -> &str {
    path.rsplit(['\\', '/'])
        .find(|s| !s.is_empty())
        .unwrap_or(path)
}

fn same_dir(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn png_names(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.to_ascii_lowercase().ends_with(".png"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::open(&dir.path().join("texto"), &dir.path().join("media")).unwrap();
        (dir, s)
    }

    fn texts(items: &[Item]) -> Vec<&str> {
        items
            .iter()
            .map(|i| i.text.as_deref().unwrap_or(""))
            .collect()
    }

    fn recent() -> ListFilter {
        ListFilter::default()
    }

    #[test]
    fn duplicate_text_bumps_used_at_without_new_row() {
        let (_d, s) = store();
        let a = s.add_text("hola", Some("Brave")).unwrap();
        let other = s.add_text("adiós", None).unwrap();
        let b = s.add_text("hola", Some("Word")).unwrap();
        assert_eq!(a.id, b.id);
        assert!(b.used_at > a.used_at);
        assert_eq!(b.created_at, a.created_at);
        assert_eq!(b.source_app.as_deref(), Some("Brave"));
        let all = s.list(&recent()).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, a.id);
        assert_eq!(all[1].id, other.id);
    }

    #[test]
    fn text_differing_only_in_whitespace_is_not_a_duplicate() {
        let (_d, s) = store();
        let a = s.add_text("hola", None).unwrap();
        let b = s.add_text("hola ", None).unwrap();
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn empty_text_is_rejected() {
        let (_d, s) = store();
        assert!(s.add_text("", None).is_err());
        assert!(s.add_text("  \n\t ", None).is_err());
    }

    #[test]
    fn link_detection() {
        let (_d, s) = store();
        let kind = |t: &str| s.add_text(t, None).unwrap().kind;
        assert_eq!(kind("https://example.com/a?b=1#c"), Kind::Link);
        assert_eq!(kind("  http://example.org  \n"), Kind::Link);
        assert_eq!(kind("HTTPS://Example.com"), Kind::Link);
        assert_eq!(kind("mira https://example.com"), Kind::Text);
        assert_eq!(kind("https://a.com https://b.com"), Kind::Text);
        assert_eq!(kind("https://"), Kind::Text);
        assert_eq!(kind("https:///ruta"), Kind::Text);
        assert_eq!(kind("ftp://example.com"), Kind::Text);
        assert_eq!(kind("example.com"), Kind::Text);
        let link = s.add_text("https://example.com/x", None).unwrap();
        assert_eq!(link.text.as_deref(), Some("https://example.com/x"));
    }

    #[test]
    fn preview_is_first_300_chars() {
        let (_d, s) = store();
        let long = "ñ".repeat(500);
        let item = s.add_text(&long, None).unwrap();
        assert_eq!(item.preview.chars().count(), 300);
        assert_eq!(item.text.as_deref(), Some(long.as_str()));
    }

    fn six_items(s: &Store) -> Vec<Item> {
        (1..=6)
            .map(|i| s.add_text(&format!("t{i}"), None).unwrap())
            .collect()
    }

    #[test]
    fn prune_keeps_favorites_and_spaced_items() {
        let (_d, s) = store();
        let items = six_items(&s);
        let space = s.create_space("Trabajo", "blue").unwrap();
        s.set_favorite(items[0].id, true).unwrap();
        s.set_space(items[1].id, Some(space.id)).unwrap();
        s.prune(2).unwrap();
        let left = s.list(&recent()).unwrap();
        assert_eq!(texts(&left), ["t6", "t5", "t2", "t1"]);
        s.prune(0).unwrap();
        let left = s.list(&recent()).unwrap();
        assert_eq!(texts(&left), ["t2", "t1"]);
    }

    #[test]
    fn prune_under_limit_deletes_nothing() {
        let (_d, s) = store();
        six_items(&s);
        s.prune(6).unwrap();
        assert_eq!(s.list(&recent()).unwrap().len(), 6);
    }

    #[test]
    fn clear_history_keeps_favorites_and_spaced_items() {
        let (_d, s) = store();
        let items = six_items(&s);
        let space = s.create_space("Casa", "green").unwrap();
        s.set_favorite(items[2].id, true).unwrap();
        s.set_space(items[4].id, Some(space.id)).unwrap();
        s.clear_history().unwrap();
        let left = s.list(&recent()).unwrap();
        assert_eq!(texts(&left), ["t5", "t3"]);
    }

    #[test]
    fn deleting_a_space_leaves_its_items_without_space() {
        let (_d, s) = store();
        let a = s.add_text("uno", None).unwrap();
        let b = s.add_text("dos", None).unwrap();
        let space = s.create_space("Viajes", "teal").unwrap();
        let keep = s.create_space("Otro", "red").unwrap();
        s.set_space(a.id, Some(space.id)).unwrap();
        s.set_space(b.id, Some(keep.id)).unwrap();
        s.delete_space(space.id).unwrap();
        assert_eq!(s.get(a.id).unwrap().space_id, None);
        assert_eq!(s.get(b.id).unwrap().space_id, Some(keep.id));
        assert_eq!(s.spaces().unwrap(), vec![keep]);
        s.clear_history().unwrap();
        assert_eq!(texts(&s.list(&recent()).unwrap()), ["dos"]);
    }

    #[test]
    fn space_crud_validates_input() {
        let (_d, s) = store();
        assert!(s.create_space("  ", "blue").is_err());
        assert!(s.create_space("X", "magenta").is_err());
        let sp = s.create_space("  Trabajo ", "orange").unwrap();
        assert_eq!(sp.name, "Trabajo");
        s.rename_space(sp.id, "Oficina").unwrap();
        assert_eq!(s.spaces().unwrap()[0].name, "Oficina");
        assert!(s.rename_space(999, "Nada").is_err());
        let it = s.add_text("x", None).unwrap();
        assert!(s.set_space(it.id, Some(999)).is_err());
        assert!(s.set_space(999, Some(sp.id)).is_err());
    }

    #[test]
    fn duplicate_image_by_hash_and_png_removed_with_last_user() {
        let (_d, s) = store();
        let png = b"\x89PNG fake image bytes";
        let a = s.add_image(png, 1920, 1080, Some("Paint")).unwrap();
        let b = s.add_image(png, 1920, 1080, None).unwrap();
        assert_eq!(a.id, b.id);
        assert!(b.used_at > a.used_at);
        assert_eq!(a.kind, Kind::Image);
        assert_eq!(a.preview, "1920×1080");
        assert_eq!(a.text, None);
        let file = a.image_file.clone().unwrap();
        assert_eq!(file.len(), 64 + 4);
        let path = s.media_dir().join(&file);
        assert_eq!(fs::read(&path).unwrap(), png);
        assert_eq!(s.image_path(a.id).unwrap(), path);

        s.conn
            .execute(
                "INSERT INTO items (kind, preview, image_file, created_at, used_at)
                 VALUES ('image', '1x1', ?1, 1, 1)",
                [&file],
            )
            .unwrap();
        let second = s.conn.last_insert_rowid();
        s.delete(a.id).unwrap();
        assert!(path.exists(), "otro elemento aún usa el PNG");
        s.delete(second).unwrap();
        assert!(!path.exists());
        assert!(s.list(&recent()).unwrap().is_empty());
    }

    #[test]
    fn pruning_and_clearing_remove_orphan_pngs() {
        let (_d, s) = store();
        let a = s.add_image(b"uno", 1, 1, None).unwrap();
        let b = s.add_image(b"dos", 1, 1, None).unwrap();
        let c = s.add_image(b"tres", 1, 1, None).unwrap();
        s.set_favorite(a.id, true).unwrap();
        s.prune(1).unwrap();
        let exists = |i: &Item| s.media_dir().join(i.image_file.as_ref().unwrap()).exists();
        assert!(exists(&a));
        assert!(!exists(&b));
        assert!(exists(&c));
        s.clear_history().unwrap();
        assert!(exists(&a));
        assert!(!exists(&c));
    }

    #[test]
    fn list_by_view_kind_query_and_space() {
        let (_d, s) = store();
        let a = s.add_text("Receta de paella", None).unwrap();
        let b = s.add_text("https://example.com/paella", None).unwrap();
        let c = s.add_text("factura 2026", None).unwrap();
        let img = s.add_image(b"img", 10, 20, None).unwrap();
        let space = s.create_space("Cocina", "orange").unwrap();
        s.set_favorite(a.id, true).unwrap();
        s.set_favorite(c.id, true).unwrap();
        s.set_space(a.id, Some(space.id)).unwrap();
        s.set_space(b.id, Some(space.id)).unwrap();

        let ids = |f: ListFilter| s.list(&f).unwrap().iter().map(|i| i.id).collect::<Vec<_>>();
        let f = |view: &str| ListFilter {
            view: view.into(),
            ..Default::default()
        };

        assert_eq!(ids(recent()), [img.id, c.id, b.id, a.id]);
        assert_eq!(ids(f("favorites")), [c.id, a.id]);
        assert_eq!(
            ids(ListFilter {
                space_id: Some(space.id),
                ..f("space")
            }),
            [b.id, a.id]
        );
        assert!(ids(f("space")).is_empty());
        assert_eq!(
            ids(ListFilter {
                kind: Some(Kind::Link),
                ..recent()
            }),
            [b.id]
        );
        assert_eq!(
            ids(ListFilter {
                query: Some("PAELLA".into()),
                ..recent()
            }),
            [b.id, a.id]
        );
        assert_eq!(
            ids(ListFilter {
                query: Some("paella".into()),
                ..f("favorites")
            }),
            [a.id]
        );
        assert_eq!(
            ids(ListFilter {
                query: Some("10×20".into()),
                ..recent()
            }),
            [img.id]
        );
        assert_eq!(
            ids(ListFilter {
                query: Some("  ".into()),
                ..recent()
            })
            .len(),
            4
        );
        assert_eq!(
            ids(ListFilter {
                limit: Some(2),
                ..recent()
            }),
            [img.id, c.id]
        );
        assert!(s.list(&f("todo")).is_err());
    }

    #[test]
    fn query_ignores_case_with_accents() {
        let (_d, s) = store();
        s.add_text("El Árbol más ÑANDÚ", None).unwrap();
        let q = |q: &str| {
            s.list(&ListFilter {
                query: Some(q.into()),
                ..recent()
            })
            .unwrap()
            .len()
        };
        assert_eq!(q("árbol"), 1);
        assert_eq!(q("ÁRBOL"), 1);
        assert_eq!(q("ñandú"), 1);
        assert_eq!(q("arbol"), 0);
    }

    #[test]
    fn files_dedupe_by_list_and_search_by_path() {
        let (_d, s) = store();
        let paths = vec![
            "C:\\Users\\usuario\\Docs\\Informe.pdf".to_string(),
            "C:\\Users\\usuario\\foto.png".to_string(),
        ];
        let a = s.add_files(&paths, Some("Explorador")).unwrap();
        assert_eq!(a.kind, Kind::Files);
        assert_eq!(a.preview, "Informe.pdf, foto.png");
        assert_eq!(a.files, paths);
        assert_eq!(a.text, None);
        let b = s.add_files(&paths, None).unwrap();
        assert_eq!(a.id, b.id);
        let reversed: Vec<String> = paths.iter().rev().cloned().collect();
        assert_ne!(s.add_files(&reversed, None).unwrap().id, a.id);
        assert!(s.add_files(&[], None).is_err());
        let found = s
            .list(&ListFilter {
                query: Some("c:\\users\\usuario\\docs".into()),
                ..recent()
            })
            .unwrap();
        assert_eq!(found.len(), 2);
        assert!(found.iter().any(|i| i.files == paths));
        let by_name = s
            .list(&ListFilter {
                query: Some("FOTO.PNG".into()),
                ..recent()
            })
            .unwrap();
        assert_eq!(by_name.len(), 2);
        let none = s
            .list(&ListFilter {
                query: Some("otra.docx".into()),
                ..recent()
            })
            .unwrap();
        assert!(none.is_empty());
    }

    #[test]
    fn secrets_are_not_deduped_and_keep_their_bytes() {
        let (_d, s) = store();
        let a = s.add_secret(&[1, 2, 3], Some("Bitwarden")).unwrap();
        let b = s.add_secret(&[1, 2, 3], Some("Bitwarden")).unwrap();
        assert_ne!(a.id, b.id);
        assert_eq!(a.kind, Kind::Secret);
        assert_eq!(a.preview, "••••••••");
        assert_eq!(a.text, None);
        assert_eq!(a.image_file, None);
        assert!(a.files.is_empty());
        assert_eq!(s.secret_blob(a.id).unwrap(), vec![1, 2, 3]);
        let plain = s.add_text("hola", None).unwrap();
        assert!(s.secret_blob(plain.id).is_err());
        assert!(s.secret_blob(999).is_err());
        assert!(s.add_secret(&[], None).is_err());
        let only = s
            .list(&ListFilter {
                kind: Some(Kind::Secret),
                ..recent()
            })
            .unwrap();
        assert_eq!(only.len(), 2);
        let stored_text: Option<String> = s
            .conn
            .query_row("SELECT text FROM items WHERE id = ?1", [a.id], |r| r.get(0))
            .unwrap();
        assert_eq!(stored_text, None);
    }

    #[test]
    fn touch_favorite_and_missing_ids() {
        let (_d, s) = store();
        let a = s.add_text("a", None).unwrap();
        let b = s.add_text("b", None).unwrap();
        s.set_favorite(a.id, true).unwrap();
        assert!(s.get(a.id).unwrap().favorite);
        assert_eq!(s.get(a.id).unwrap().used_at, a.used_at);
        s.touch(a.id).unwrap();
        assert_eq!(s.list(&recent()).unwrap()[0].id, a.id);
        s.delete(b.id).unwrap();
        s.delete(b.id).unwrap();
        assert!(s.get(b.id).is_err());
        assert!(s.touch(b.id).is_err());
        assert!(s.set_favorite(b.id, true).is_err());
    }

    #[test]
    fn schema_is_versioned_and_persists_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let (t, m) = (dir.path().join("t"), dir.path().join("m"));
        let s = Store::open(&t, &m).unwrap();
        let version: i64 = s
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, MIGRATIONS.len() as i64);
        let item = s.add_text("persistente", None).unwrap();
        drop(s);
        let s = Store::open(&t, &m).unwrap();
        assert_eq!(s.get(item.id).unwrap().text.as_deref(), Some("persistente"));
        s.conn.pragma_update(None, "user_version", 99).unwrap();
        drop(s);
        assert!(Store::open(&t, &m).is_err());
    }

    #[test]
    fn move_text_dir_moves_database_and_keeps_working() {
        let dir = tempfile::tempdir().unwrap();
        let (old, new, media) = (
            dir.path().join("viejo"),
            dir.path().join("nuevo"),
            dir.path().join("m"),
        );
        let mut s = Store::open(&old, &media).unwrap();
        let a = s.add_text("antes", None).unwrap();
        let space = s.create_space("Espacio", "pink").unwrap();
        s.set_space(a.id, Some(space.id)).unwrap();

        s.move_text_dir(&new).unwrap();
        assert!(new.join("pegatina.db").exists());
        assert!(!old.join("pegatina.db").exists());
        assert_eq!(s.text_dir(), new);
        assert_eq!(s.get(a.id).unwrap().space_id, Some(space.id));
        let b = s.add_text("después", None).unwrap();
        drop(s);

        let s = Store::open(&new, &media).unwrap();
        assert_eq!(texts(&s.list(&recent()).unwrap()), ["después", "antes"]);
        assert_eq!(s.get(b.id).unwrap().text.as_deref(), Some("después"));
        assert_eq!(s.spaces().unwrap(), vec![space]);
    }

    #[test]
    fn move_text_dir_to_same_dir_or_existing_db_is_safe() {
        let dir = tempfile::tempdir().unwrap();
        let (a_dir, b_dir, media) = (
            dir.path().join("a"),
            dir.path().join("b"),
            dir.path().join("m"),
        );
        Store::open(&b_dir, &media).unwrap();
        let mut s = Store::open(&a_dir, &media).unwrap();
        s.add_text("x", None).unwrap();
        s.move_text_dir(&a_dir).unwrap();
        assert!(s.move_text_dir(&b_dir).is_err());
        assert_eq!(s.text_dir(), a_dir);
        assert_eq!(s.list(&recent()).unwrap().len(), 1);
        assert!(a_dir.join("pegatina.db").exists());
    }

    #[test]
    fn move_media_dir_moves_pngs_and_keeps_working() {
        let dir = tempfile::tempdir().unwrap();
        let (text, old, new) = (
            dir.path().join("t"),
            dir.path().join("viejo"),
            dir.path().join("nuevo"),
        );
        let mut s = Store::open(&text, &old).unwrap();
        let a = s.add_image(b"primera", 4, 4, None).unwrap();
        let file_a = a.image_file.clone().unwrap();
        fs::write(old.join("notas.txt"), "no es mía").unwrap();

        s.move_media_dir(&new).unwrap();
        assert!(!old.join(&file_a).exists());
        assert_eq!(fs::read(new.join(&file_a)).unwrap(), b"primera");
        assert!(old.join("notas.txt").exists());
        assert_eq!(s.media_dir(), new);
        assert_eq!(s.image_path(a.id).unwrap(), new.join(&file_a));

        let b = s.add_image(b"segunda", 4, 4, None).unwrap();
        assert!(new.join(b.image_file.as_ref().unwrap()).exists());
        assert!(!old.join(b.image_file.as_ref().unwrap()).exists());
        s.delete(a.id).unwrap();
        assert!(!new.join(&file_a).exists());
    }

    #[test]
    fn move_media_dir_removes_emptied_old_dir_and_ignores_same_dir() {
        let dir = tempfile::tempdir().unwrap();
        let (text, old, new) = (
            dir.path().join("t"),
            dir.path().join("viejo"),
            dir.path().join("nuevo"),
        );
        let mut s = Store::open(&text, &old).unwrap();
        let a = s.add_image(b"x", 1, 1, None).unwrap();
        s.move_media_dir(&old).unwrap();
        assert!(old.join(a.image_file.as_ref().unwrap()).exists());
        s.move_media_dir(&new).unwrap();
        assert!(!old.exists());
    }
}
