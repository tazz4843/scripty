use scripty_macros::handle_serenity_error;
use serenity::{
    builder::CreateEmbed,
    client::Context,
    framework::standard::{macros::command, CommandResult},
    model::channel::Message,
};

#[command("donate")]
#[aliases("premium")]
#[bucket = "general"]
#[description = "Find out how to donate, help support bot development, and get some perks in return."]
async fn cmd_donate(ctx: &Context, msg: &Message) -> CommandResult {
    let mut embed = CreateEmbed::default();
    embed.title("Donating");
    embed.description("Donating helps pay for Scripty's server costs (which are higher than you might think because it needs GPUs)\n\
    You can donate at https://github.com/sponsors/tazz4843");
    embed.field("Current Donors", "1 anonymous", true);
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
