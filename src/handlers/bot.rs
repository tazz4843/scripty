use crate::globals::START_TIME;
use crate::{auto_join, globals::BotConfig, metrics_counter, utils};
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::id::GuildId,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};
use tracing::info;

/// The event handler struct to implement EventHandler for
pub struct Handler {
    pub is_loop_running: AtomicBool,
    pub start_time: SystemTime,
}

#[async_trait]
/// The implementation you should add your own event handling functions to
impl EventHandler for Handler {
    /// Triggered when the bot is ready or added to a guild
    /// - Prints the number of guilds the bot is in and DMs the owner using `log()`
    /// # Panics
    /// If setting it failed, meaning BotInfo wasn't initialised
    async fn cache_ready(&self, ctx: Context, guilds: Vec<GuildId>) {
        if let Some(config) = BotConfig::get() {
            if config.log_guild_added() {
                let msg = format!("In {} guilds!", guilds.len());
                info!("{}", msg);
                crate::log(&ctx, msg).await;
            }
        } else {
            crate::log(
                &ctx,
                "Couldn't get BotConfig to see if guild adds should be added",
            )
            .await
        }
        // it's safe to clone Context, but Arc is cheaper for this use case.
        // Untested claim, just theoretically. :P
        let ctx = Arc::new(ctx);

        // We need to check that the loop is not already running when this event triggers,
        // as this event triggers every time the bot enters or leaves a guild, along every time the
        // ready shard event triggers.
        //
        // An AtomicBool is used because it doesn't require a mutable reference to be changed, as
        // we don't have one due to self being an immutable reference.
        if !self.is_loop_running.load(Ordering::Relaxed) {
            if let Err(_) = START_TIME.set(self.start_time.into()) {
                return;
            };
            self.is_loop_running.swap(true, Ordering::Relaxed);
            info!(
                "Started client in {}ms!",
                self.start_time
                    .elapsed()
                    .expect("System clock rolled back!")
                    .as_millis()
            );

            // We have to clone the Arc, as it gets moved into the new thread.
            let ctx1 = Arc::clone(&ctx);
            let ctx2 = Arc::clone(&ctx);
            let ctx3 = Arc::clone(&ctx);
            let ctx4 = Arc::clone(&ctx);
            // tokio::spawn creates a new green thread that can run in parallel with the rest of
            // the application.
            tokio::spawn(async move {
                loop {
                    utils::do_stats_update(Arc::clone(&ctx1)).await;
                    tokio::time::sleep(Duration::from_secs(30)).await;
                }
            });

            tokio::spawn(async move {
                loop {
                    auto_join::auto_join(Arc::clone(&ctx2)).await;
                    tokio::time::sleep(Duration::from_secs(300)).await;
                }
            });

            tokio::spawn(async move {
                loop {
                    metrics_counter::metrics_counter(Arc::clone(&ctx3)).await;
                    tokio::time::sleep(Duration::from_secs(300)).await;
                }
            });

            tokio::spawn(async move {
                loop {
                    utils::update_status(Arc::clone(&ctx4)).await;
                    tokio::time::sleep(Duration::from_secs(30)).await
                }
            });
        }
    }
}
