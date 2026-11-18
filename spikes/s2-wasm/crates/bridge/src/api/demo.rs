//! API du pont : couche anticorruption mince au-dessus de `s2_core`.
use futures_util::StreamExt;

use crate::frb_generated::StreamSink;

/// Catégorie d'erreur (enum C-like : pas besoin de `freezed` côté Dart).
#[derive(Debug, Clone, Copy)]
pub enum BridgeErrorKind {
    InvalidInput,
    Sink,
    /// Erreur SQLite.
    Store,
    /// Stockage persistant indisponible dans ce contexte.
    StoreUnavailable,
    /// Un autre onglet tient déjà le stockage persistant.
    StoreOpenInAnotherTab,
}

/// Erreur exposée à Dart (DTO simple ; devient une exception Dart).
#[derive(Debug)]
pub struct BridgeError {
    pub kind: BridgeErrorKind,
    pub message: String,
}

impl From<s2_core::CoreError> for BridgeError {
    fn from(e: s2_core::CoreError) -> Self {
        match e {
            s2_core::CoreError::InvalidInput(message) => Self {
                kind: BridgeErrorKind::InvalidInput,
                message,
            },
        }
    }
}

/// Appel async : somme après un court délai.
///
/// DTO en `i32`/`u32` : sur le web, FRB traduit `i64`/`u64` en `BigInt` alors
/// qu'en natif `i64` devient `int` ; le même code Dart ne compilerait pas partout.
pub async fn delayed_sum(a: i32, b: i32, delay_ms: u32) -> Result<i32, BridgeError> {
    let sum = s2_core::delayed_sum(i64::from(a), i64::from(b), u64::from(delay_ms)).await?;
    i32::try_from(sum).map_err(|_| BridgeError {
        kind: BridgeErrorKind::InvalidInput,
        message: "dépassement i32".into(),
    })
}

/// Flux : pousse `count` ticks dans le `StreamSink`, puis le ferme.
pub async fn tick_stream(
    sink: StreamSink<u32>,
    count: u32,
    period_ms: u32,
) -> Result<(), BridgeError> {
    let mut ticks = Box::pin(s2_core::ticks(count, u64::from(period_ms)));
    while let Some(i) = ticks.next().await {
        sink.add(i).map_err(|e| BridgeError {
            kind: BridgeErrorKind::Sink,
            message: format!("{e:?}"),
        })?;
    }
    Ok(())
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}
