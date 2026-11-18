//! API d'essai S5 : émetteurs de flux instrumentés.
//!
//! Chaque flux porte un `stream_id` choisi par Dart. Le registre `REGISTRY` garde, pour chaque
//! identifiant, le nombre d'émissions, l'état (en cours ou non) et la raison d'arrêt : c'est ce
//! qui permet de vérifier, côté Dart, que l'émetteur Rust s'arrête après une annulation.
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use crate::frb_generated::StreamSink;

/// Événement émis : identifiant du flux et numéro de séquence (0, 1, 2…).
#[derive(Debug, Clone)]
pub struct FluxEvent {
    pub stream_id: u32,
    pub seq: u32,
}

/// Pourquoi un émetteur s'est arrêté.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// Encore en cours.
    None,
    /// Toutes les émissions demandées ont été faites.
    Completed,
    /// `sink.add` a renvoyé une erreur (Dart ne reçoit plus).
    SendError,
    /// Arrêt explicite demandé par Dart (`stop_stream`).
    StopRequested,
    /// L'émetteur a terminé en renvoyant une erreur.
    Failed,
}

/// État d'un émetteur, lu par Dart.
#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub stream_id: u32,
    pub known: bool,
    pub running: bool,
    pub emitted: u32,
    pub stop_reason: StopReason,
}

/// Catégorie d'erreur (enum C-like : pas besoin de `freezed` côté Dart).
#[derive(Debug, Clone, Copy)]
pub enum BridgeErrorKind {
    InvalidInput,
    Sink,
    Simulated,
}

/// Erreur exposée à Dart (devient une exception Dart implémentant `FrbException`).
#[derive(Debug, Clone)]
pub struct BridgeError {
    pub kind: BridgeErrorKind,
    pub message: String,
}

/// Comment `error_stream` signale son erreur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorMode {
    /// `sink.add_error(...)` au milieu du flux, puis on continue.
    AddErrorThenContinue,
    /// La fonction renvoie `Err(...)` après quelques émissions.
    ReturnErr,
}

#[derive(Debug, Clone)]
struct Entry {
    running: bool,
    emitted: u32,
    stop_reason: StopReason,
    stop_requested: bool,
}

static REGISTRY: Mutex<Option<HashMap<u32, Entry>>> = Mutex::new(None);

fn registry() -> MutexGuard<'static, Option<HashMap<u32, Entry>>> {
    lock_spin(&REGISTRY)
}

/// Verrou sans attente bloquante : en WASM threadé, `Mutex::lock` contendu appelle
/// `memory.atomic.wait`, interdit sur le fil principal du navigateur (« Atomics.wait cannot
/// be called in this context »). On boucle donc sur `try_lock` (sections critiques courtes).
pub(crate) fn lock_spin<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    loop {
        match m.try_lock() {
            Ok(g) => return g,
            Err(std::sync::TryLockError::Poisoned(p)) => return p.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => std::hint::spin_loop(),
        }
    }
}

fn with_entry<R>(stream_id: u32, f: impl FnOnce(&mut Entry) -> R) -> R {
    let mut guard = registry();
    let map = guard.get_or_insert_with(HashMap::new);
    let entry = map.entry(stream_id).or_insert(Entry {
        running: false,
        emitted: 0,
        stop_reason: StopReason::None,
        stop_requested: false,
    });
    f(entry)
}

fn start(stream_id: u32) {
    with_entry(stream_id, |e| {
        *e = Entry {
            running: true,
            emitted: 0,
            stop_reason: StopReason::None,
            stop_requested: false,
        }
    });
}

fn finish(stream_id: u32, reason: StopReason) {
    with_entry(stream_id, |e| {
        e.running = false;
        e.stop_reason = reason;
    });
}

/// Émet un événement ; renvoie la raison d'arrêter s'il faut arrêter.
fn emit(sink: &StreamSink<FluxEvent>, stream_id: u32, seq: u32) -> Option<StopReason> {
    if with_entry(stream_id, |e| e.stop_requested) {
        return Some(StopReason::StopRequested);
    }
    if sink.add(FluxEvent { stream_id, seq }).is_err() {
        return Some(StopReason::SendError);
    }
    with_entry(stream_id, |e| e.emitted += 1);
    None
}

/// Rafale depuis une fonction `async` : `count` émissions sans attente entre elles.
/// Sur le web, les fonctions `async` s'exécutent sur le fil principal (`spawn_local`).
pub async fn burst_async(
    sink: StreamSink<FluxEvent>,
    stream_id: u32,
    count: u32,
) -> Result<(), BridgeError> {
    start(stream_id);
    for seq in 0..count {
        if let Some(reason) = emit(&sink, stream_id, seq) {
            finish(stream_id, reason);
            return Ok(());
        }
    }
    finish(stream_id, StopReason::Completed);
    Ok(())
}

/// Rafale depuis une fonction normale (non `async`) : sur le web threadé et en natif, elle
/// s'exécute dans le pool de fils (Web Worker) ; avec `default_dart_async: false`, en
/// synchrone sur le fil principal.
pub fn burst_sync(
    sink: StreamSink<FluxEvent>,
    stream_id: u32,
    count: u32,
) -> Result<(), BridgeError> {
    start(stream_id);
    for seq in 0..count {
        if let Some(reason) = emit(&sink, stream_id, seq) {
            finish(stream_id, reason);
            return Ok(());
        }
    }
    finish(stream_id, StopReason::Completed);
    Ok(())
}

/// Émetteur cadencé `async` : une émission toutes les `period_ms`. `count == 0` : infini,
/// jusqu'à ce que `sink.add` échoue ou que Dart appelle `stop_stream`.
pub async fn paced_async(
    sink: StreamSink<FluxEvent>,
    stream_id: u32,
    count: u32,
    period_ms: u32,
) -> Result<(), BridgeError> {
    start(stream_id);
    let mut seq = 0u32;
    while count == 0 || seq < count {
        if let Some(reason) = emit(&sink, stream_id, seq) {
            finish(stream_id, reason);
            return Ok(());
        }
        seq = seq.wrapping_add(1);
        s5_core::sleep_ms(period_ms).await;
    }
    finish(stream_id, StopReason::Completed);
    Ok(())
}

/// Émetteur cadencé dans un fil du pool (bloquant, `std::thread::sleep`). À ne pas appeler
/// avec `default_dart_async: false` : il bloquerait le fil principal.
pub fn paced_worker(
    sink: StreamSink<FluxEvent>,
    stream_id: u32,
    count: u32,
    period_ms: u32,
) -> Result<(), BridgeError> {
    start(stream_id);
    let mut seq = 0u32;
    while count == 0 || seq < count {
        if let Some(reason) = emit(&sink, stream_id, seq) {
            finish(stream_id, reason);
            return Ok(());
        }
        seq = seq.wrapping_add(1);
        std::thread::sleep(std::time::Duration::from_millis(u64::from(period_ms)));
    }
    finish(stream_id, StopReason::Completed);
    Ok(())
}

/// Émet 0, 1, 2 puis signale une erreur selon `mode` ; en `AddErrorThenContinue`, émet
/// ensuite 3 et 4 et se termine normalement.
pub async fn error_stream(
    sink: StreamSink<FluxEvent>,
    stream_id: u32,
    mode: ErrorMode,
) -> Result<(), BridgeError> {
    start(stream_id);
    for seq in 0..3 {
        if let Some(reason) = emit(&sink, stream_id, seq) {
            finish(stream_id, reason);
            return Ok(());
        }
    }
    let err = BridgeError {
        kind: BridgeErrorKind::Simulated,
        message: "erreur simulée".into(),
    };
    match mode {
        ErrorMode::ReturnErr => {
            finish(stream_id, StopReason::Failed);
            Err(err)
        }
        ErrorMode::AddErrorThenContinue => {
            // Le côté Dart décode toute erreur de flux comme `AnyhowException` (une chaîne) :
            // passer `BridgeError` tel quel donnerait un message illisible. On encode donc le
            // code stable dans la chaîne.
            if sink.add_error(format!("{:?}:{}", err.kind, err.message)).is_err() {
                finish(stream_id, StopReason::SendError);
                return Ok(());
            }
            for seq in 3..5 {
                if let Some(reason) = emit(&sink, stream_id, seq) {
                    finish(stream_id, reason);
                    return Ok(());
                }
            }
            finish(stream_id, StopReason::Completed);
            Ok(())
        }
    }
}

/// Demande l'arrêt coopératif d'un émetteur (annulation explicite depuis Dart).
#[flutter_rust_bridge::frb(sync)]
pub fn stop_stream(stream_id: u32) {
    with_entry(stream_id, |e| e.stop_requested = true);
}

/// État d'un émetteur.
#[flutter_rust_bridge::frb(sync)]
pub fn stream_info(stream_id: u32) -> StreamInfo {
    let guard = registry();
    match guard.as_ref().and_then(|m| m.get(&stream_id)) {
        Some(e) => StreamInfo {
            stream_id,
            known: true,
            running: e.running,
            emitted: e.emitted,
            stop_reason: e.stop_reason,
        },
        None => StreamInfo {
            stream_id,
            known: false,
            running: false,
            emitted: 0,
            stop_reason: StopReason::None,
        },
    }
}

/// Nombre d'émetteurs encore en cours (tous identifiants confondus).
#[flutter_rust_bridge::frb(sync)]
pub fn running_streams() -> u32 {
    let guard = registry();
    guard
        .as_ref()
        .map(|m| m.values().filter(|e| e.running).count())
        .and_then(|n| u32::try_from(n).ok())
        .unwrap_or(u32::MAX)
}

/// Appel ponctuel `async` (contrôle de vie du pont).
pub async fn ping(delay_ms: u32) -> Result<u32, BridgeError> {
    s5_core::sleep_ms(delay_ms).await;
    Ok(delay_ms)
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}
