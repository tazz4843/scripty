use crate::utils::ShardManagerWrapper;
use serenity::{
    client::Context,
    framework::standard::{macros::command, CommandResult},
    model::prelude::Message,
};
use std::hint::unreachable_unchecked;

#[command("shutdown")]
#[description = "Begins the bot shutdown process. This command might not update depending on \
whether the stack overflows before exit."]
#[owners_only]
async fn cmd_shutdown(ctx: &Context, msg: &Message) -> CommandResult {
    if handle_message!(ctx, msg, |m| m.content("Beginning shutdown...")).is_none() {
        return Ok(());
    }
    let data = ctx.data.write().await;
    let manager = data
        .get::<ShardManagerWrapper>()
        .unwrap_or_else(|| unsafe { unreachable_unchecked() });
    let manager = manager.write().await;
    manager.lock().await.shutdown_all().await;

    if handle_message!(ctx, msg, |m| m.content("All shards shutting down...")).is_none() {
        return Ok(());
    }

    Ok(())
}
