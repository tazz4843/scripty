use crate::globals::PgPoolKey;
use crate::{bind, globals::BotConfig, utils::do_stats_update};
use serenity::futures::TryStreamExt;
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{
        gateway::{Activity, Ready},
        id::GuildId,
    },
};
use std::{
    hint,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};

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
                println!("{}", msg);
                crate::log(&ctx, msg).await;
            }
        } else {
            {
                crate::log(
                    &ctx,
                    "Couldn't get BotConfig to see if guild adds should be added",
                )
                .await
            }
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
            // We have to clone the Arc, as it gets moved into the new thread.
            let ctx1 = Arc::clone(&ctx);
            let ctx2 = Arc::clone(&ctx);
            // tokio::spawn creates a new green thread that can run in parallel with the rest of
            // the application.
            tokio::spawn(async move {
                loop {
                    // We clone Context again here, because Arc is owned, so it moves to the
                    // new function.
                    do_stats_update(Arc::clone(&ctx1)).await;
                    tokio::time::sleep(Duration::from_secs(30)).await;
                }
            });

            tokio::spawn(async move {
                loop {
                    let data = ctx2.data.read().await;
                    let pool = data.get::<PgPoolKey>().unwrap_or_else(|| unsafe {
                        hint::unreachable_unchecked()
                        // SAFETY: this should absolutely never happen if the DB pool is placed
                        // in at initialization. if that were to happen, undefined behavior would result anyways
                    });
                    let mut query = sqlx::query!("SELECT * FROM guilds").fetch(pool);
                    loop {
                        match query.try_next().await {
                            Ok(row) => match row {
                                Some(row) => {
                                    let guild_id = row.guild_id;

                                    if let Some(_) = songbird::get(&ctx2)
                                        .await
                                        .unwrap_or_else(|| unsafe {
                                            hint::unreachable_unchecked() // SAFETY: this should absolutely never happen if Songbird is registered at client init.
                                                                          // if it isn't registered, UB would result anyways
                                        })
                                        .get::<u64>(guild_id as u64)
                                    {
                                        continue;
                                    };

                                    let vc_id = match row.default_bind {
                                        Some(v) => v,
                                        None => { continue; }
                                    };
                                    let result_id = match row.output_channel {
                                        Some(v) => v,
                                        None => { continue; }
                                    };

                                    let _ = bind::bind(
                                        &ctx,
                                        (vc_id as u64).into(),
                                        (result_id as u64).into(),
                                        (guild_id as u64).into(),
                                    )
                                    .await;
                                }
                                None => {
                                    break;
                                }
                            },
                            Err(_) => {
                                continue;
                            }
                        }
                    }

                    tokio::time::sleep(Duration::from_secs(300)).await;
                }
            });

            // Now that the loop is running, we set the bool to true
            self.is_loop_running.swap(true, Ordering::Relaxed);
        }
    }

    /// Triggered when the bot or a new shard is ready
    /// - Sets the activity of the bot to `@{bot username} help`
    async fn ready(&self, ctx: Context, info: Ready) {
        println!(
            "Started client in {}ms!",
            self.start_time
                .elapsed()
                .expect("System clock rolled back!")
                .as_millis()
        );
        ctx.set_activity(Activity::playing(
            format!("@{} help", info.user.name).as_str(),
        ))
        .await;
    }
}
