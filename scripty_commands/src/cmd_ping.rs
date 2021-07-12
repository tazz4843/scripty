use scripty_db::PgPoolKey;
use scripty_macros::handle_serenity_error;
use scripty_utils::{get_avg_ws_latency, ContextTypes};
use serenity::model::prelude::GuildId;
use serenity::{
    builder::CreateEmbed,
    client::Context,
    framework::standard::{macros::command, CommandResult},
    model::prelude::Message,
};
use sqlx::query;
use std::time::SystemTime;

#[command("ping")]
#[aliases("p")]
#[bucket = "general"]
#[description = "Play a game of ping-pong!"]
async fn cmd_ping(ctx: &Context, msg: &Message) -> CommandResult {
    let (ws_latency, _) = get_avg_ws_latency(ContextTypes::NoArc(ctx)).await;
    let rest_api_latency = {
        let st = SystemTime::now();
        msg.channel_id.broadcast_typing(&ctx.http).await?;
        st.elapsed()?.as_nanos() as f64
    };
    let db_latency = {
        let data = ctx.data.read().await;
        let db = unsafe { data.get::<PgPoolKey>().unwrap_unchecked() };
        let guild_id = *msg.guild_id.unwrap_or(GuildId(675390855716274216)).as_u64();
        let st = SystemTime::now();
        query!("SELECT * FROM guilds WHERE guild_id = $1", guild_id as i64)
            .fetch_optional(db)
            .await?;
        st.elapsed()?.as_nanos() as f64
    };
    let mut embed = CreateEmbed::default();
    embed.title("🏓");
    embed.field("WebSocket", format!("{}ms", ws_latency), false);
    embed.field(
        "Discord REST API",
        format!("{}ms", rest_api_latency / 1_000_000.0),
        false,
    );
    embed.field("PSQL", format!("{}ms", db_latency / 1_000_000.0), false);
    if let Err(e) = msg
        .channel_id
        .send_message(&ctx, |m| {
            m.embed(|e| {
                *e = embed;
                e
            })
        })
        .await
    {
        handle_serenity_error!(e);
    }

    Ok(())
}
