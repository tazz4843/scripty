use crate::{ContextTypes, ShardManagerWrapper};

/// Gets the average websocket latency.
pub async fn get_avg_ws_latency(ctx: ContextTypes<'_>) -> (u128, u8) {
    let c = match ctx {
        ContextTypes::NoArc(c) => c,
        ContextTypes::WithArc(c) => c,
    };
    let data_read = c.data.read().await;

    let shard_manager_lock = data_read
        .get::<ShardManagerWrapper>()
        .expect("Expected shard manager in data map.")
        .clone();
    let shard_manager_guard = shard_manager_lock.read().await;
    let shard_manager = shard_manager_guard.lock().await;
    let mut total: u8 = 0;
    let mut latency: u128 = 0;
    for i in shard_manager.runners.lock().await.iter() {
        if let Some(l) = i.1.latency {
            total += 1;
            latency += l.as_millis();
        }
    }
    if total == 0 {
        // no shards ready
        latency = 0
    } else {
        latency /= total as u128; // scales to a arbitrary number of shards well
    }
    (latency, total)
}
