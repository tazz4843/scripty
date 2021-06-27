use crate::{
     send_embed,
    utils::{get_avg_ws_latency, ContextTypes},
};
use serenity::{
    builder::CreateEmbed,
    client::Context,
    framework::standard::{macros::command, CommandResult},
    model::prelude::Message,
};

#[command("ping")]
#[aliases("p")]
#[bucket = "general"]
#[description = "Play a game of ping-pong!"]
async fn cmd_ping(ctx: &Context, msg: &Message) -> CommandResult {
    let latency = get_avg_ws_latency(ContextTypes::NoArc(ctx)).await;
    let start = std::time::SystemTime::now();
    msg.channel_id.broadcast_typing(&ctx.http).await?;
    let ping_time = start.elapsed()?.as_millis();
    let mut embed = CreateEmbed::default();
    embed.title("🏓");
    embed.field("WebSocket", format!("{}ms", latency.0), false);
    embed.field("Discord REST API", format!("{}ms", ping_time), false);
    send_embed(ctx, msg, false, embed).await;
    Ok(())
}
