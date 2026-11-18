//! Cœur factice de l'essai S1 : aucune dépendance au pont.
use std::time::Duration;

use futures_core::Stream;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    InvalidInput(String),
}

/// Fonction async : renvoie `a + b` après un court délai.
pub async fn delayed_sum(a: i64, b: i64, delay_ms: u64) -> Result<i64, CoreError> {
    if delay_ms > 10_000 {
        return Err(CoreError::InvalidInput(format!(
            "délai trop long : {delay_ms} ms"
        )));
    }
    futures_timer::Delay::new(Duration::from_millis(delay_ms)).await;
    a.checked_add(b)
        .ok_or_else(|| CoreError::InvalidInput("dépassement entier".into()))
}

/// Source d'événements périodiques : `count` ticks espacés de `period_ms`.
pub fn ticks(count: u32, period_ms: u64) -> impl Stream<Item = u32> {
    async_stream::stream! {
        for i in 0..count {
            futures_timer::Delay::new(Duration::from_millis(period_ms)).await;
            yield i;
        }
    }
}
