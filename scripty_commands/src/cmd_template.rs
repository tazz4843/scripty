use scripty_macros::handle_serenity_error;
use serenity::{
    builder::CreateEmbed,
    client::Context,
    framework::standard::{macros::command, CommandResult},
    model::prelude::Message,
};

#[command("template")]
#[aliases("tmp")]
#[bucket = "general"]
#[description = "Template command: not to be used."]
async fn cmd_ping(ctx: &Context, msg: &Message) -> CommandResult {
    let mut embed = CreateEmbed::default();
    embed.title("Template Command");
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
