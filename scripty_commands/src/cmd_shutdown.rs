use scripty_macros::handle_serenity_error;
use scripty_utils::ShardManagerWrapper;
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
    if let Err(e) = msg
        .channel_id
        .send_message(ctx, |m| m.content("Beginning shutdown..."))
        .await
    {
        handle_serenity_error!(e);
        return Ok(());
    }
    let data = ctx.data.write().await;
    let manager = data
        .get::<ShardManagerWrapper>()
        .unwrap_or_else(|| unsafe { unreachable_unchecked() });
    let manager = manager.write().await;
    manager.lock().await.shutdown_all().await;

    if let Err(e) = msg
        .channel_id
        .send_message(&ctx, |m| m.content("All shards shut down."))
        .await
    {
        handle_serenity_error!(e);
    }

    Ok(())
}
