//! Port de persistance et adaptateur SQLite (essai S2).
//!
//! Le même code `rusqlite` sert en natif et sur `wasm32-unknown-unknown`.
use rusqlite::Connection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    /// Erreur SQLite (message de `rusqlite`).
    Sqlite(String),
    /// Le stockage persistant est déjà tenu par un autre onglet.
    AlreadyOpenElsewhere,
    /// Le stockage persistant n'est pas disponible dans ce contexte.
    Unavailable(String),
}

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e.to_string())
    }
}

/// Port : un carnet de notes minimal.
pub trait NoteStore {
    fn add(&mut self, text: &str) -> Result<i64, StoreError>;
    fn list(&self) -> Result<Vec<String>, StoreError>;
    /// Insère `count` notes dans une seule transaction ; renvoie le total en base.
    fn add_many(&mut self, prefix: &str, count: u32) -> Result<i64, StoreError>;
}

/// Adaptateur SQLite du port [`NoteStore`].
pub struct SqliteNoteStore {
    conn: Connection,
}

impl SqliteNoteStore {
    /// Base en mémoire (perdue à la fermeture).
    pub fn open_in_memory() -> Result<Self, StoreError> {
        Self::init(Connection::open_in_memory()?)
    }

    /// Base nommée, ouverte avec le VFS par défaut de la plateforme :
    /// fichier en natif ; sur le web, le VFS mémoire de `sqlite-wasm-rs`,
    /// ou `sahpool` (OPFS) s'il a été installé comme VFS par défaut.
    pub fn open(path: &str) -> Result<Self, StoreError> {
        Self::init(Connection::open(path)?)
    }

    fn init(conn: Connection) -> Result<Self, StoreError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS note (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 text TEXT NOT NULL
             );",
        )?;
        Ok(Self { conn })
    }
}

impl NoteStore for SqliteNoteStore {
    fn add(&mut self, text: &str) -> Result<i64, StoreError> {
        self.conn
            .execute("INSERT INTO note (text) VALUES (?1)", [text])?;
        Ok(self.conn.last_insert_rowid())
    }

    fn list(&self) -> Result<Vec<String>, StoreError> {
        let mut stmt = self.conn.prepare("SELECT text FROM note ORDER BY id")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    fn add_many(&mut self, prefix: &str, count: u32) -> Result<i64, StoreError> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare("INSERT INTO note (text) VALUES (?1)")?;
            for i in 0..count {
                stmt.execute([format!("{prefix}-{i}")])?;
            }
        }
        tx.commit()?;
        Ok(self
            .conn
            .query_row("SELECT count(*) FROM note", [], |row| row.get(0))?)
    }
}

/// Prépare le stockage persistant de la plateforme avant [`SqliteNoteStore::open`].
///
/// Natif : rien à faire. Web : prend (si demandé) le verrou d'onglet unique,
/// puis installe `sahpool` comme VFS par défaut. Doit être appelée depuis le
/// fil qui utilisera ensuite la connexion.
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
pub async fn prepare_persistent(_tab_lock: bool) -> Result<(), StoreError> {
    Ok(())
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub use web::prepare_persistent;

/// Décrit le contexte d'exécution courant (diagnostic de l'essai).
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
pub fn execution_context() -> String {
    let thread = std::thread::current();
    format!("fil natif « {} »", thread.name().unwrap_or("sans nom"))
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub use web::execution_context;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
mod web {
    use js_sys::{Function, Promise, Reflect};
    use sqlite_wasm_rs::WasmOsCallback;
    use sqlite_wasm_vfs::sahpool::{install, OpfsSAHPoolCfg};
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;

    use super::StoreError;

    /// Nom du verrou Web Locks qui garantit un seul onglet propriétaire d'OPFS.
    const TAB_LOCK: &str = "s2-notes-db";

    pub async fn prepare_persistent(tab_lock: bool) -> Result<(), StoreError> {
        if tab_lock && !acquire_tab_lock().await? {
            return Err(StoreError::AlreadyOpenElsewhere);
        }
        install::<WasmOsCallback>(&OpfsSAHPoolCfg::default(), true)
            .await
            .map(|_| ())
            .map_err(|e| StoreError::Unavailable(format!("sahpool : {e} ({e:?})")))
    }

    /// Demande le verrou sans attendre (`ifAvailable`). S'il est obtenu, il est
    /// tenu jusqu'à la mort du contexte (promesse jamais résolue).
    ///
    /// `inline_js` n'est pas utilisable avec `--target no-modules` (imposé par
    /// FRB) : la fonction JS est donc construite à l'exécution.
    async fn acquire_tab_lock() -> Result<bool, StoreError> {
        let js = Function::new_with_args(
            "name",
            "if (!(self.navigator && navigator.locks)) return Promise.reject(new Error('Web Locks indisponible'));
             return new Promise((resolve, reject) => {
               navigator.locks.request(name, { ifAvailable: true }, (lock) => {
                 if (!lock) { resolve(false); return; }
                 resolve(true);
                 return new Promise(() => {});
               }).catch(reject);
             });",
        );
        let promise = js
            .call1(&JsValue::NULL, &JsValue::from_str(TAB_LOCK))
            .map_err(js_error)?
            .dyn_into::<Promise>()
            .map_err(js_error)?;
        let got = JsFuture::from(promise).await.map_err(js_error)?;
        Ok(got.as_bool().unwrap_or(false))
    }

    pub fn execution_context() -> String {
        let global = js_sys::global();
        let scope = global.constructor().name();
        let isolated = Reflect::get(&global, &JsValue::from_str("crossOriginIsolated"))
            .ok()
            .and_then(|v| v.as_bool());
        format!("{scope} (crossOriginIsolated={isolated:?})")
    }

    fn js_error(e: JsValue) -> StoreError {
        StoreError::Unavailable(format!("{e:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecrit_puis_relit() -> Result<(), StoreError> {
        let mut store = SqliteNoteStore::open_in_memory()?;
        assert_eq!(store.add("un")?, 1);
        assert_eq!(store.add("deux")?, 2);
        assert_eq!(store.list()?, vec!["un".to_owned(), "deux".to_owned()]);
        Ok(())
    }
}
