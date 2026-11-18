//! Repli prévu par l'étude (§5.4) : modèle de lecture servi par requête ponctuelle, plus un
//! flux de simples notifications de changement (numéro de version, sans données).
//!
//! Une notification perdue ou fusionnée n'est pas grave : Dart relit l'instantané complet, qui
//! porte la version. Le flux ne transporte donc aucune donnée métier.
use std::sync::{Mutex, MutexGuard};

use crate::frb_generated::StreamSink;

/// Instantané du modèle de lecture.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub version: u32,
    pub items: Vec<String>,
}

struct Store {
    version: u32,
    items: Vec<String>,
    watchers: Vec<(u32, StreamSink<u32>)>,
}

static STORE: Mutex<Option<Store>> = Mutex::new(None);

fn store() -> MutexGuard<'static, Option<Store>> {
    crate::api::flux::lock_spin(&STORE)
}

fn with_store<R>(f: impl FnOnce(&mut Store) -> R) -> R {
    let mut guard = store();
    let s = guard.get_or_insert_with(|| Store {
        version: 0,
        items: Vec::new(),
        watchers: Vec::new(),
    });
    f(s)
}

/// Commande : ajoute un élément et notifie les observateurs (version seule).
#[flutter_rust_bridge::frb(sync)]
pub fn repli_add(item: String) -> u32 {
    with_store(|s| {
        s.items.push(item);
        s.version += 1;
        let v = s.version;
        // Un observateur dont l'envoi échoue est retiré (natif). Sur le web l'envoi
        // n'échoue pas : c'est `repli_unwatch` qui retire l'observateur.
        s.watchers.retain(|(_, sink)| sink.add(v).is_ok());
        v
    })
}

/// Requête ponctuelle : instantané complet.
#[flutter_rust_bridge::frb(sync)]
pub fn repli_get() -> Snapshot {
    with_store(|s| Snapshot {
        version: s.version,
        items: s.items.clone(),
    })
}

/// Réinitialise le modèle (tests).
#[flutter_rust_bridge::frb(sync)]
pub fn repli_reset() {
    with_store(|s| {
        s.version = 0;
        s.items.clear();
        s.watchers.clear();
    });
}

/// Abonnement aux notifications de changement. La fonction rend la main tout de suite ; le
/// `StreamSink` est conservé dans le magasin.
#[flutter_rust_bridge::frb(sync)]
pub fn repli_watch(watch_id: u32, sink: StreamSink<u32>) {
    with_store(|s| {
        let v = s.version;
        // Première notification : la version courante, pour déclencher la lecture initiale.
        if sink.add(v).is_ok() {
            s.watchers.push((watch_id, sink));
        }
    });
}

/// Désabonnement explicite (à appeler depuis `onCancel` côté Dart).
#[flutter_rust_bridge::frb(sync)]
pub fn repli_unwatch(watch_id: u32) {
    with_store(|s| s.watchers.retain(|(id, _)| *id != watch_id));
}

/// Nombre d'observateurs enregistrés (détection de fuite).
#[flutter_rust_bridge::frb(sync)]
pub fn repli_watchers() -> u32 {
    with_store(|s| u32::try_from(s.watchers.len()).unwrap_or(u32::MAX))
}
