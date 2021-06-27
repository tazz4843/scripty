use crate::{bind, globals::PgPoolKey};
use serenity::{futures::TryStreamExt, model::id::ChannelId, prelude::Context};
use std::{convert::TryInto, sync::Arc};
use tracing::{debug, warn};

/// Automatically joins all voice chats the bot can see in its DB.
/// `ctx` is a Arc<Context> containing the DB pool and Songbird client
/// `force` decides whether to forcibly rejoin a voice chat. This will result in errors at some point.
pub async fn auto_join(ctx: Arc<Context>, force: bool) {
    let data = ctx.data.read().await;
    let pool = unsafe { data.get::<PgPoolKey>().unwrap_unchecked() };
    let mut query = sqlx::query!("SELECT * FROM guilds").fetch(pool);
    while let Ok(row) = query.try_next().await {
        match row {
            Some(row) => {
                let guild_id = row.guild_id;

                unsafe {
                    if !force {
                        let already_connected = songbird::get(&ctx)
                            .await
                            .unwrap_unchecked()
                            .get::<u64>(guild_id as u64)
                            .is_some();
                        if already_connected {
                            continue;
                        };
                    }
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

                if let Err(e) = bind::bind(
                    &ctx,
                    (vc_id as u64).into(),
                    (result_id as u64).into(),
                    (guild_id as u64).into(),
                )
                .await
                {
                    warn!("failed to join VC in {}: {}", guild_id, e);
                    if let Err(e) = ChannelId(result_id.try_into().unwrap()).send_message(&ctx, |m | {
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
                    }).await {
                        warn!("couldn't warn users about error in {}: {}", guild_id, e);
                        // if these queries fail so be it
                        let _ = sqlx::query!("DELETE FROM guilds WHERE guild_id = $1", guild_id).execute(pool).await;
                        let _ = sqlx::query!("DELETE FROM channels WHERE channel_id = $1", result_id).execute(pool).await;
                    }
                } else {
                    debug!("joined VC in {} successfully", guild_id);
                };
            }
            None => {
                break;
            }
        }
    }
}
