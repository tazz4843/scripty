use crate::bind;
use scripty_db::PG_POOL;
use serenity::{futures::TryStreamExt, model::id::ChannelId, prelude::Context};
use std::{convert::TryInto, sync::Arc};
use tracing::{debug, info, warn};

/// Automatically joins all voice chats the bot can see in its DB.
/// `ctx` is a Arc<Context> containing the DB pool and Songbird client
/// `force` decides whether to forcibly rejoin a voice chat. This will result in errors at some point.
pub async fn auto_join(ctx: Arc<Context>, force: bool) {
    let pool = unsafe { PG_POOL.get().unwrap_unchecked() };
    let mut query = sqlx::query!("SELECT * FROM guilds").fetch(pool);
    debug!("Connecting to all VCs");
    let mut vcs = Vec::new();
    for row in query.try_next().await {
        match row {
            Some(row) => vcs.push(row),
            _ => break,
        }
    }
    for row in vcs {
        let guild_id = row.guild_id;
        info!(guild_id = guild_id, "trying to connect to VC");

        if force {
            debug!(
                guild_id = guild_id,
                "force set to true, not checking if a connection exists",
            );
        } else {
            let already_connected = unsafe {
                songbird::get(&ctx)
                    .await
                    .unwrap_unchecked()
                    .get::<u64>(guild_id as u64)
                    .is_some()
            };
            if already_connected {
                debug!(guild_id = guild_id, "connection already exists, skipping");
                continue;
            };
        }

        let vc_id = match row.default_bind {
            Some(v) => v,
            None => {
                continue;
            }
        };
        let result_id = match row.output_channel {
            Some(v) => v,
            None => {
                continue;
            }
        };

        if let Err(e) = bind(
            &ctx,
            (vc_id as u64).into(),
            (result_id as u64).into(),
            (guild_id as u64).into(),
        )
        .await
        {
            warn!(guild_id = guild_id, "failed to join VC: {}", e);
            if let Err(e) =
                ChannelId(result_id.try_into().unwrap())
                    .send_message(&ctx, |m| {
                        m.embed(|embed| {
                            embed
                        .color(11534368)
                        .description(format!("I can't join the voice chat you have set up! {}", e))
                        .field("Need help fixing it?", "https://discord.gg/xSpNJSjNhq", true)
                        .footer(|c| {
                            c.text("This message will continually be sent until this is fixed.")
                        })
                        .title("Error while joining VC!")
                        })
                    })
                    .await
            {
                warn!(
                    guild_id = guild_id,
                    "couldn't warn users about error: {}", e
                );
                // if these queries fail so be it
                let _ = sqlx::query!("DELETE FROM guilds WHERE guild_id = $1", guild_id)
                    .execute(pool)
                    .await;
                let _ = sqlx::query!("DELETE FROM channels WHERE channel_id = $1", result_id)
                    .execute(pool)
                    .await;
            }
        } else {
            debug!(guild_id, "joined VC successfully");
        };
    }
}
