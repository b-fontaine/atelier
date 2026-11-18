//! Cœur factice de l'essai S5 : un minuterie portable, aucune dépendance au pont.
use std::time::Duration;

/// Attente asynchrone portable (natif : fil de futures-timer ; WASM : setTimeout).
pub async fn sleep_ms(ms: u32) {
    futures_timer::Delay::new(Duration::from_millis(u64::from(ms))).await;
}
