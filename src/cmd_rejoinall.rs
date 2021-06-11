use serenity::{
    client::Context,
    framework::standard::{macros::command, CommandResult},
    model::prelude::Message,
};
use crate::{auto_join, msg_handler::handle_message};
use std::sync::Arc;

#[command("rejoin_all")]
#[description = "Forces the bot to rejoin every single voice chat it is in."]
#[owners_only]
async fn cmd_ping(ctx: &Context, msg: &Message) -> CommandResult {
    let mut msg1 = match handle_message(ctx, msg, |m| {
        m.content("Reconnecting to all voice chats...")
    }).await {
        Some(m) => m,
        None => return Ok(())
    };
    let _typing = msg.channel_id.start_typing(ctx.as_ref()).expect("failed to start typing");
    auto_join::auto_join(Arc::new(ctx.clone())).await;
    let _ = msg1.edit(ctx, |m| {
        m.content("Reconnected to all voice chats.")
    }).await;
    Ok(())
}
