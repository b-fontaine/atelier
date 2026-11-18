//! API du pont : couche anticorruption mince au-dessus de `s1_core`.
use futures_util::StreamExt;

use crate::frb_generated::StreamSink;

/// Catégorie d'erreur (enum C-like : pas besoin de `freezed` côté Dart).
#[derive(Debug, Clone, Copy)]
pub enum BridgeErrorKind {
    InvalidInput,
    Sink,
}

/// Erreur exposée à Dart (DTO simple ; devient une exception Dart).
#[derive(Debug)]
pub struct BridgeError {
    pub kind: BridgeErrorKind,
    pub message: String,
}

impl From<s1_core::CoreError> for BridgeError {
    fn from(e: s1_core::CoreError) -> Self {
        match e {
            s1_core::CoreError::InvalidInput(message) => Self {
                kind: BridgeErrorKind::InvalidInput,
                message,
            },
        }
    }
}

/// Appel async : somme après un court délai.
pub async fn delayed_sum(a: i64, b: i64, delay_ms: u64) -> Result<i64, BridgeError> {
    Ok(s1_core::delayed_sum(a, b, delay_ms).await?)
}

/// Flux : pousse `count` ticks dans le `StreamSink`, puis le ferme.
pub async fn tick_stream(
    sink: StreamSink<u32>,
    count: u32,
    period_ms: u64,
) -> Result<(), BridgeError> {
    let mut ticks = Box::pin(s1_core::ticks(count, period_ms));
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
