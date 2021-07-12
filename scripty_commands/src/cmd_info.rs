use scripty_config::BotConfig;
use scripty_macros::handle_serenity_error;
use scripty_utils::BotInfo;
use serenity::{
    builder::CreateEmbed,
    client::Context,
    framework::standard::{macros::command, CommandResult},
    model::prelude::{Mentionable, Message},
};

#[command("info")]
#[aliases("about", "invite", "inv")]
#[bucket = "general"]
#[description = "How you can add me to your server, contact my owner, find my GitHub page etc."]
async fn cmd_info(ctx: &Context, msg: &Message) -> CommandResult {
    let mut embed = CreateEmbed::default();
    embed.footer(|f| {
        f.text("I act weirdly? Want me to speak another language? Anything else? You can join the support server anytime for any feedback you have!")
    });

    match BotInfo::get() {
        Some(info) => {
            embed
                .description(&info.description())
                .field("Made by:", info.owner().mention(), true);
        }
        None => {
            tracing::info!("Couldn't get BotInfo for the `info` command");
            embed.description("Awkward but I think I forgot who I am..");
        }
    };

    match BotConfig::get() {
        Some(config) => {
            embed
                .title("Want me in your server? Click here then!")
                .url(&config.invite())
                .field("on GitHub:", &config.github(), true);
        }
        None => {
            tracing::info!("Couldn't get BotConfig for the `info` command");
            embed.title("Oops, I lost my invite, I swear I had it right here");
        }
    };
    embed.field("Support Server", "https://discord.gg/VT7EgQ3RQW", true);
    embed.field("Bot Version", env!("CARGO_PKG_VERSION"), true);
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
