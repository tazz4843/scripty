use crate::send_embed;
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
    send_embed(ctx, msg, false, embed).await;
    Ok(())
}
