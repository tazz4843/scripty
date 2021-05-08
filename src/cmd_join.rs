use crate::{bind, log, send_embed};
use serenity::{
    builder::CreateEmbed,
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::{channel::Message, id::ChannelId},
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
                .reply(ctx, "The snowflake ID you gave was invalid.")
                .await
            {
                log(ctx, format!("Failed to send message! {:?}", e)).await
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
    let mut is_error = false;

    embed.description(
        match bind::bind(ctx, bind_channel, transcription_channel, guild_id).await {
            Ok(_) => {
                format!(
                    "Joined {} and bound to {} successfully.",
                    bind_channel.mention(),
                    transcription_channel.mention()
                )
            }
            Err(e) => {
                is_error = true;
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
    send_embed(ctx, msg, is_error, embed).await;

    Ok(())
}
