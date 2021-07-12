use scripty_audio::bind;
use scripty_macros::handle_serenity_error;
use serenity::{
    builder::CreateEmbed,
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::{ChannelId, Message},
    prelude::Mentionable,
};
use tracing::error;

#[command("join")]
#[required_permissions("MANAGE_GUILD")]
#[only_in("guilds")]
#[bucket = "expensive"]
#[description = "Bind the bot to a voice channel. Only really useful for debugging.\n**This command \
must be used in a very specific way. Doing otherwise will result in errors.**"]
async fn cmd_join(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let bind_channel = match args.single::<u64>() {
        Ok(id) => ChannelId(id),
        Err(_) => {
            if let Err(e) = msg
                .channel_id
                .send_message(&ctx, |m| {
                    m.content("The snowflake ID you gave was invalid.")
                })
                .await
            {
                handle_serenity_error!(e);
            }
            return Ok(());
        }
    };
    let guild_id = msg.guild_id.unwrap_or_else(|| {
        error!("somehow in DMs for join cmd");
        panic!("somehow in DMs")
    });
    let transcription_channel = msg.channel_id;

    let mut embed = CreateEmbed::default();

    embed.description(
        match bind(ctx, bind_channel, transcription_channel, guild_id).await {
            Ok(_) => {
                format!(
                    "Joined {} and bound to {} successfully.",
                    bind_channel.mention(),
                    transcription_channel.mention()
                )
            }
            Err(e) => {
                let err = format!(
                    "Failed to join {} because {}",
                    transcription_channel.mention(),
                    e
                );
                error!("{}", err);
                err
            }
        },
    );
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
