use crate::ShardManagerWrapper;
use rand::Rng;
use serenity::client::bridge::gateway::ShardRunnerMessage;
use serenity::model::prelude::{Activity, OnlineStatus};
use serenity::prelude::Context;
use std::sync::Arc;
use tracing::error;

const STATUSES: [OnlineStatus; 3] = [
    OnlineStatus::Online,
    OnlineStatus::Idle,
    OnlineStatus::DoNotDisturb,
];

pub async fn update_status(ctx: Arc<Context>) {
    let guild_count = ctx.cache.guild_count().await;

    let data_read = ctx.data.read().await;
    let shard_manager_lock = data_read
        .get::<ShardManagerWrapper>()
        .expect("Expected shard manager in data map.")
        .clone();
    let shard_manager_guard = shard_manager_lock.read().await;
    let shard_manager = shard_manager_guard.lock().await;

    for i in shard_manager.runners.lock().await.iter() {
        let latency = match i.1.latency {
            Some(l) => l.as_micros() as f64 / 1000_f64,
            None => 0_f64,
        };
        let activity = Activity::playing(format!(
            "Shard {} | {}ms latency | {} servers | ~setup to get started",
            i.0 .0, latency, guild_count
        ));

        let mut rng = rand::thread_rng();
        let status = unsafe { STATUSES.get(rng.gen_range(0..=2)).unwrap_unchecked() };

        if let Err(e) =
            i.1.runner_tx
                .send_to_shard(ShardRunnerMessage::SetPresence(*status, Some(activity)))
        {
            error!("failed to update status on shard {}: {}", i.0 .0, e);
        };
    }
}
