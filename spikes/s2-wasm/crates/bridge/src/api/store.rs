//! API de stockage : un acteur propriétaire de la connexion SQLite, sur un fil dédié.
//!
//! Sur le web, les `async fn` du pont s'exécutent sur le fil principal, où
//! OPFS (`createSyncAccessHandle`) est interdit. La connexion vit donc dans un
//! Worker du pool FRB ; le pont lui envoie des commandes par canal. Le même
//! schéma sert en natif (fil dédié), ce qui garde un seul chemin de code.
use std::future::Future;
use std::sync::Mutex;

use futures_channel::{mpsc, oneshot};
use futures_util::StreamExt;
use s2_core::store::{self, NoteStore, SqliteNoteStore, StoreError};

use super::demo::{BridgeError, BridgeErrorKind};

#[derive(Debug, Clone, Copy)]
pub enum StoreMode {
    /// Base SQLite en mémoire.
    Memory,
    /// Base persistante : fichier en natif, OPFS (`sahpool`) sur le web.
    Persistent,
}

/// Diagnostic renvoyé à l'ouverture.
#[derive(Debug)]
pub struct StoreInfo {
    /// Contexte où tourne l'appel du pont (`async fn`).
    pub bridge_context: String,
    /// Contexte où vit la connexion SQLite.
    pub store_context: String,
}

enum Command {
    Open(
        StoreMode,
        String,
        bool,
        oneshot::Sender<Result<String, StoreError>>,
    ),
    Close(oneshot::Sender<()>),
    Add(String, oneshot::Sender<Result<i64, StoreError>>),
    List(oneshot::Sender<Result<Vec<String>, StoreError>>),
    AddMany(String, u32, oneshot::Sender<Result<i64, StoreError>>),
}

/// Canal vers l'unique fil de stockage du processus (ou de la page web).
static STORE: Mutex<Option<mpsc::UnboundedSender<Command>>> = Mutex::new(None);

impl From<StoreError> for BridgeError {
    fn from(e: StoreError) -> Self {
        let (kind, message) = match e {
            StoreError::Sqlite(m) => (BridgeErrorKind::Store, m),
            StoreError::AlreadyOpenElsewhere => (
                BridgeErrorKind::StoreOpenInAnotherTab,
                "le stockage est déjà ouvert dans un autre onglet".to_owned(),
            ),
            StoreError::Unavailable(m) => (BridgeErrorKind::StoreUnavailable, m),
        };
        Self { kind, message }
    }
}

fn worker_gone() -> BridgeError {
    BridgeError {
        kind: BridgeErrorKind::StoreUnavailable,
        message: "le fil de stockage ne répond plus".to_owned(),
    }
}

/// Renvoie le canal du fil de stockage, en le démarrant au premier appel.
///
/// Un seul fil pour toute la vie de la page : `sahpool` est lié au Worker qui
/// l'a installé (ses poignées OPFS sont des objets JS de ce Worker), alors que
/// le registre de VFS de SQLite est global à la mémoire partagée.
async fn store_worker() -> Result<mpsc::UnboundedSender<Command>, BridgeError> {
    if let Some(tx) = STORE.lock().map_err(|_| worker_gone())?.as_ref() {
        return Ok(tx.clone());
    }
    let (tx, rx) = mpsc::unbounded();
    spawn_store_worker(move || actor(rx)).await?;
    *STORE.lock().map_err(|_| worker_gone())? = Some(tx.clone());
    Ok(tx)
}

async fn ask<T>(make: impl FnOnce(oneshot::Sender<T>) -> Command) -> Result<T, BridgeError> {
    let (tx, rx) = oneshot::channel();
    store_worker()
        .await?
        .unbounded_send(make(tx))
        .map_err(|_| worker_gone())?;
    rx.await.map_err(|_| worker_gone())
}

/// Ouvre le stockage. `db_name` : chemin de fichier en natif, nom logique sur le web.
/// `tab_lock` : prendre le verrou d'onglet unique avant d'installer OPFS (web).
pub async fn store_open(
    mode: StoreMode,
    db_name: String,
    tab_lock: bool,
) -> Result<StoreInfo, BridgeError> {
    let store_context = ask(|reply| Command::Open(mode, db_name, tab_lock, reply)).await??;
    Ok(StoreInfo {
        bridge_context: store::execution_context(),
        store_context,
    })
}

/// Ferme la connexion (le fil de stockage et le verrou d'onglet restent en place).
pub async fn store_close() -> Result<(), BridgeError> {
    ask(Command::Close).await
}

pub async fn store_add(text: String) -> Result<i32, BridgeError> {
    let id = ask(|reply| Command::Add(text, reply)).await??;
    i32::try_from(id).map_err(|_| worker_gone())
}

pub async fn store_list() -> Result<Vec<String>, BridgeError> {
    Ok(ask(Command::List).await??)
}

/// Charge : `count` insertions en une transaction ; renvoie le total en base.
pub async fn store_add_many(prefix: String, count: u32) -> Result<i32, BridgeError> {
    let total = ask(|reply| Command::AddMany(prefix, count, reply)).await??;
    i32::try_from(total).map_err(|_| worker_gone())
}

/// Fonction non async : FRB l'exécute dans un Worker du pool (web) ou un fil
/// du pool (natif). Sert à charger l'allocateur partagé pendant que SQLite
/// (compilé `SQLITE_THREADSAFE=0`) travaille dans un autre Worker.
pub fn churn_allocator(rounds: u32) -> u32 {
    let mut total = 0u32;
    for round in 0..rounds {
        let strings: Vec<String> = (0..256).map(|i| format!("{round}-{i}")).collect();
        total = total.wrapping_add(strings.iter().map(|s| s.len() as u32).sum::<u32>());
    }
    total
}

fn not_open() -> StoreError {
    StoreError::Unavailable("stockage non ouvert".to_owned())
}

/// Boucle de l'acteur : seul propriétaire de la connexion, sur un seul fil.
async fn actor(mut commands: mpsc::UnboundedReceiver<Command>) {
    let mut notes: Option<SqliteNoteStore> = None;
    let mut persistent_ready = false;
    while let Some(command) = commands.next().await {
        match command {
            Command::Open(mode, db_name, tab_lock, reply) => {
                // `sahpool` n'accepte qu'une connexion par base : fermer l'ancienne.
                notes = None;
                let opened = match mode {
                    StoreMode::Memory => SqliteNoteStore::open_in_memory(),
                    StoreMode::Persistent => {
                        let prepared = if persistent_ready {
                            Ok(())
                        } else {
                            store::prepare_persistent(tab_lock).await
                        };
                        match prepared {
                            Ok(()) => {
                                persistent_ready = true;
                                SqliteNoteStore::open(&db_name)
                            }
                            Err(e) => Err(e),
                        }
                    }
                };
                let _ = reply.send(match opened {
                    Ok(store) => {
                        notes = Some(store);
                        Ok(store::execution_context())
                    }
                    Err(e) => Err(e),
                });
            }
            Command::Close(reply) => {
                notes = None;
                let _ = reply.send(());
            }
            Command::Add(text, reply) => {
                let _ = reply.send(match notes.as_mut() {
                    Some(n) => n.add(&text),
                    None => Err(not_open()),
                });
            }
            Command::List(reply) => {
                let _ = reply.send(match notes.as_ref() {
                    Some(n) => n.list(),
                    None => Err(not_open()),
                });
            }
            Command::AddMany(prefix, count, reply) => {
                let _ = reply.send(match notes.as_mut() {
                    Some(n) => n.add_many(&prefix, count),
                    None => Err(not_open()),
                });
            }
        }
    }
}

/// Natif : un fil dédié qui exécute l'acteur jusqu'à la fermeture du canal.
#[cfg(not(target_family = "wasm"))]
async fn spawn_store_worker<F, Fut>(make: F) -> Result<(), BridgeError>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + 'static,
{
    std::thread::Builder::new()
        .name("s2-store".to_owned())
        .spawn(move || futures_executor::block_on(make()))
        .map(|_| ())
        .map_err(|e| BridgeError {
            kind: BridgeErrorKind::StoreUnavailable,
            message: e.to_string(),
        })
}

/// Web : l'acteur est lancé dans un Worker du pool FRB, puis y reste (boucle
/// d'événements du Worker). Le futur n'est pas `Send` (objets JS) : il est
/// construit dans le Worker.
#[cfg(target_family = "wasm")]
async fn spawn_store_worker<F, Fut>(make: F) -> Result<(), BridgeError>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + 'static,
{
    flutter_rust_bridge::spawn_blocking_with(
        move || {
            reserve_current_worker();
            wasm_bindgen_futures::spawn_local(make());
        },
        crate::frb_generated::FLUTTER_RUST_BRIDGE_HANDLER.thread_pool(),
    )
    .await
    .map_err(|_| worker_gone())
}

/// Retire le Worker courant du pool FRB (contournement, dépend d'un détail
/// interne de FRB 2.13.0).
///
/// Le pool est une pile : dès que la closure se termine, le Worker envoie un
/// message vide au fil principal, qui le remet en tête des Workers libres. La
/// prochaine fonction non async du pont y serait donc exécutée et bloquerait
/// la boucle d'événements de l'acteur SQLite pendant toute sa durée. On masque
/// `postMessage` pour ce seul envoi : le pool ne récupère jamais ce Worker et
/// en crée un autre au besoin.
#[cfg(target_family = "wasm")]
fn reserve_current_worker() {
    let swallow_once = js_sys::Function::new_no_args("delete self.postMessage;");
    let _ = js_sys::Reflect::set(
        &js_sys::global(),
        &wasm_bindgen::JsValue::from_str("postMessage"),
        &swallow_once,
    );
}
