use crate::{globals::START_TIME, metrics::Metrics};
use chrono::Utc;
use serenity::{
    client::Context,
    framework::standard::{macros::command, CommandResult},
    model::prelude::Message,
};
use std::hint::unreachable_unchecked;

#[command("stats")]
#[bucket = "expensive"]
#[description = "Live statistics on the bot."]
async fn cmd_stats(ctx: &Context, msg: &Message) -> CommandResult {
    let metrics = ctx.data.read().await.get::<Metrics>().cloned().unwrap();
    let members = metrics.members.get();
    let guilds = metrics.guilds.get();
    let messages = metrics.events.message_create.get();
    let total_events = metrics.total_events.get();
    let ms_transcribed = metrics.ms_transcribed.get();
    let total_events_per_sec = total_events
        / START_TIME
            .get()
            .unwrap_or_else(|| unsafe { unreachable_unchecked() })
            .signed_duration_since(Utc::now())
            .num_seconds() as u64;

    if let Err(e) = msg
        .channel_id
        .send_message(&ctx, |m| {
            m.embed(|e| {
                e.title("Bot Stats")
                    .field("Total Guilds", guilds, false)
                    .field("Total Users", members, false)
                    .field("Total Messages", messages, false)
                    .field("Total Gateway Events", total_events, false)
                    .field("Average Gateway Events/sec", total_events_per_sec, false)
                    .field(
                        "Seconds of Audio Processed",
                        ms_transcribed * 1000_u64,
                        false,
                    )
                    .footer(|f| f.text("All stats are since"))
                    .timestamp(
                        START_TIME
                            .get()
                            .unwrap_or_else(|| unsafe { unreachable_unchecked() }),
                    )
            })
        })
        .await
    {
        if let Err(_) = msg
            .author
            .direct_message(&ctx, |m| {
                m.content(format!(
                    "I can't send messages in <#{}> for some reason: {}",
                    msg.channel_id.as_u64(),
                    e
                ))
            })
            .await
        {
            let _ = msg.react(&ctx, '❌').await;
        };
    };

    Ok(())
}
