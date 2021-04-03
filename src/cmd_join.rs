use serenity::{
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::{
        channel::{Channel, ChannelType, Message},
        id::ChannelId,
    },
    prelude::Mentionable,
};

use crate::{globals::RedisConnectionWrapper, log, utils::Receiver};
use songbird::CoreEvent;

#[command("join")]
#[required_permissions("MANAGE_GUILD")]
#[only_in("guilds")]
#[bucket = "expensive"]
#[description = "Bind the bot to a voice channel. Only really useful for debugging."]
async fn cmd_join(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let connect_to = match args.single::<u64>() {
        Ok(id) => ChannelId(id),
        Err(_) => {
            if let Err(e) = msg
                .reply(ctx, "Requires a valid voice channel ID be given")
                .await
            {
                log(ctx, format!("Failed to send message! {:?}", e)).await
            }

            return Ok(());
        }
    };

    match match connect_to.to_channel_cached(&ctx).await {
        Some(c) => c,
        None => match connect_to.to_channel(&ctx).await {
            Ok(c) => c,
            Err(e) => {
                let _ = msg
                    .channel_id
                    .say(&ctx, format!("I can't convert that to a channel. {:?}", e))
                    .await;
                return Ok(());
            }
        },
    } {
        Channel::Guild(c) => match c.kind {
            ChannelType::Voice => {}
            ChannelType::Stage => {}
            _ => {
                let _ = msg
                    .channel_id
                    .say(&ctx, "This isn't a voice channel! Try again.")
                    .await;
                return Ok(());
            }
        },
        _ => {
            let _ = msg
                .channel_id
                .say(&ctx, "This isn't a voice channel! Try again.")
                .await;
            return Ok(());
        }
    }

    if let Err(e) = msg
        .channel_id
        .say(&ctx.http, &"Initializing voice client, please wait...")
        .await
    {
        log(ctx, format!("Failed to send message! {:?}", e)).await
    }

    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let (handler_lock, conn_result) = manager.join(guild_id, connect_to).await;

    match conn_result {
        Ok(_) => {
            // NOTE: this skips listening for the actual connection result.
            let mut handler = handler_lock.lock().await;

            let tm = ctx.data.read().await;

            let redis_conn = tm
                .get::<RedisConnectionWrapper>()
                .expect("Redis connection handle placed in at initialization.");

            let receiver = Receiver::new(redis_conn.clone());

            if let Err(e) = handler.mute(true).await {
                if let Err(e) = msg
                    .channel_id
                    .say(
                        &ctx.http,
                        &format!(
                            "Failed to mute myself! You can mute me if you desire.\nReason: {:?}",
                            e
                        ),
                    )
                    .await
                {
                    log(ctx, format!("Failed to send message! {:?}", e)).await;
                };
            };
            handler.add_global_event(CoreEvent::SpeakingStateUpdate.into(), receiver.clone());
            handler.add_global_event(CoreEvent::SpeakingUpdate.into(), receiver.clone());
            handler.add_global_event(CoreEvent::VoicePacket.into(), receiver.clone());
            handler.add_global_event(CoreEvent::ClientConnect.into(), receiver.clone());
            handler.add_global_event(CoreEvent::ClientDisconnect.into(), receiver.clone());

            if let Err(e) = msg
                .channel_id
                .say(&ctx.http, &format!("Joined {}", connect_to.mention()))
                .await
            {
                log(ctx, format!("Failed to send message! {:?}", e)).await
            }
        }
        Err(e) => {
            if let Err(e) = msg
                .channel_id
                .say(&ctx.http, format!("Error joining the channel: {}", e))
                .await
            {
                log(ctx, format!("Failed to send message! {:?}", e)).await
            }
        }
    }

    Ok(())
}
