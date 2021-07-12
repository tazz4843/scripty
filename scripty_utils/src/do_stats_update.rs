use scripty_db::PgPoolKey;
use serenity::client::Context;
use serenity::model::prelude::{ChannelId, MessageId};
use sqlx::query;
use std::sync::Arc;
use std::time::SystemTime;

pub async fn do_stats_update(ctx: Arc<Context>) {
    let shard_info = super::get_avg_ws_latency(super::ContextTypes::WithArc(&ctx)).await;

    ctx.cache.set_max_messages(0_usize).await;
    let status_channel = ChannelId(791426352217587732);
    match status_channel
        .messages(&ctx.http, |r| r.after(MessageId(0_u64)).limit(25))
        .await
    {
        Ok(m) => {
            if let Err(e) = status_channel.delete_messages(&ctx.http, m).await {
                println!("Failed to delete messages from status channel! {}", e);
            }
        }
        Err(e) => {
            println!("Failed to get most recent messages from channel! {}", e)
        }
    };

    // calculate REST API ping
    let rest_api_ping_time = {
        let start = SystemTime::now();
        if let Err(why) = status_channel.broadcast_typing(&ctx.http).await {
            println!("Failed to get latency! {}", why);
        }
        match start.elapsed() {
            Ok(t) => t.as_millis(),
            Err(e) => {
                println!("Failed to get ping time! {}", e);
                return;
            }
        }
    };

    // calculate DB ping
    let db_ping_time = {
        let mut v: u128 = 0;
        if let Some(db) = ctx.data.read().await.get::<PgPoolKey>() {
            let start = SystemTime::now();
            let _ = query!(
                "SELECT prefix FROM prefixes WHERE guild_id = $1",
                675390855716274216_i64
            )
            .fetch_optional(db)
            .await;
            v = start
                .elapsed()
                .expect("System clock rolled back!")
                .as_micros();
        }
        v
    };

    let current_name = ctx.cache.current_user().await.name;
    let guild_count = ctx.cache.guild_count().await as u64;
    let user_count = {
        let mut c: u64 = 0;
        for g in ctx.cache.guilds().await {
            if let Some(gc) = g.to_guild_cached(&ctx).await {
                c += gc.member_count;
            }
        }
        c
    };
    let avg_ws_latency = if shard_info.0 == 0 {
        "NaN".to_string()
    } else {
        shard_info.0.to_string()
    };

    if let Err(e) = status_channel
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.title(format!("{}'s status", current_name))
                    .field("Guilds in Cache", guild_count, true)
                    .field("Users in Cached Guilds", user_count, true)
                    .field("Cached Messages", 0.to_string(), true)
                    .field(
                        "Message Send Latency",
                        format!("{}ms", rest_api_ping_time),
                        true,
                    )
                    .field("Average WS Latency", format!("{}ms", avg_ws_latency), true)
                    .field("DB Query Latency", format!("{}µs", db_ping_time), true)
                    .field("Shard Count", shard_info.1, true)
                    .field(
                        "Library",
                        "[serenity-rs](https://github.com/serenity-rs/serenity)",
                        true,
                    )
                    .field(
                        "Source Code",
                        "[Click me!](https://github.com/tazz4843/scripty)",
                        true,
                    )
                    .colour(serenity::utils::Colour::ROHRKATZE_BLUE)
            })
        })
        .await
    {
        println!("Failed to update in status channel! {:?}", e);
    };
}
