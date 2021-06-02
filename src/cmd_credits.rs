use crate::send_embed;
use serenity::{
    builder::CreateEmbed,
    client::Context,
    framework::standard::{macros::command, CommandResult},
    model::channel::Message,
};

#[command("credits")]
#[bucket = "general"]
#[description = "A massive thank you to all these people/organizations. Without them, Scripty would not be possible."]
async fn cmd_credits(ctx: &Context, msg: &Message) -> CommandResult {
    let mut embed = CreateEmbed::default();
    embed.title("Credits");
    embed.description(
        "Without all these people/organizations Scripty would simply not be possible.",
    );
    embed.field(
        "DeepSpeech",
        "https://github.com/mozilla/DeepSpeech\nThe speech to text framework Scripty is running on. Funded by Mozilla.",
        false
    );
    embed.field(
        "TensorFlow",
        "https://github.com/tensorflow/tensorflow\nDeepSpeech uses this to help with the complex mathematical calculations that are required.",
        false
    );
    embed.field(
        "Common Voice",
        "https://commonvoice.mozilla.org/\nPublic audio dataset, with around 10,000 hours of audio across 30 languages",
        false
    );
    send_embed(ctx, msg, false, embed).await;
    Ok(())
}
