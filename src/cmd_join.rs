use crate::globals::SqlitePoolKey;
use crate::{handlers::audio::Receiver, log, send_embed};
use serenity::http::CacheHttp;
use serenity::{
    builder::CreateEmbed,
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::{
        channel::{Channel, ChannelType, Message},
        id::ChannelId,
    },
    prelude::Mentionable,
};
use songbird::CoreEvent;
use sqlx::sqlite::SqliteQueryResult;
use sqlx::{query, Error, Row};
use std::convert::TryInto;
use std::hint::unreachable_unchecked;
use std::sync::Arc;

#[command("join")]
#[required_permissions("MANAGE_GUILD")]
#[only_in("guilds")]
#[bucket = "expensive"]
#[description = "Bind the bot to a voice channel. Only really useful for debugging."]
async fn cmd_join(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let db = data.get::<SqlitePoolKey>();
    if db.is_none() {
        log(
            ctx,
            "Couldn't get SqlitePool for the join command".to_string(),
        )
        .await;
        let mut e = CreateEmbed::default();
        e.title("Now this is super weird and scary").description(
            "I lost my whole book where I write things down, sorry..\n
            I let my dev know, until then just please wait.",
        );
        send_embed(ctx, msg, true, e);
        return Ok(());
    };

    let connect_to = match args.single::<u64>() {
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

    match match connect_to.to_channel_cached(&ctx).await {
        Some(c) => c,
        None => match connect_to.to_channel(&ctx).await {
            Ok(c) => c,
            Err(e) => {
                msg.reply(&ctx, format!("I can't convert that to a channel. {:?}", e))
                    .await;
                return Ok(());
            }
        },
    } {
        Channel::Guild(c) => match c.kind {
            ChannelType::Voice | ChannelType::Stage => {}
            _ => {
                msg.reply(&ctx, "This isn't a voice channel! Try again.")
                    .await;
                return Ok(());
            }
        },
        _ => {
            msg.reply(&ctx, "This isn't a guild channel! Try again.")
                .await;
            return Ok(());
        }
    }

    let (token, id): (String, u64) = match query(
        "SELECT webhook_token, webhook_id FROM channels WHERE channel_id = ?",
    )
    .bind(i64::from(msg.channel_id))
    .fetch_optional(db.unwrap_or_else(|| unsafe {
        unreachable_unchecked() // why? we've already made 100% sure the DB pool exists above.
    }))
    .await
    {
        Ok(result) => {
            let result = match result {
                Some(r) => r,
                None => {
                    msg
                    .reply(
                        &ctx.http,
                        "This channel hasn't been set up! Run the `setup` command, with this channel \
                            set as the result channel."
                    )
                    .await;
                    return Ok(());
                }
            };

            let token = match result.try_get(0) {
                Ok(r) => r,
                Err(e) => {
                    msg
                    .reply(
                        &ctx.http,
                        "This channel hasn't been set up! Run the `setup` command, with this channel \
                            set as the result channel."
                    )
                    .await;
                    log(ctx, format!("Couldn't get index 0 {:?}", e)).await;
                    return Ok(());
                }
            };
            let id = match result.try_get::<i64, usize>(1) {
                Ok(r) => match r.try_into() {
                    Ok(r) => r,
                    Err(e) => {
                        msg.reply(&ctx.http, "Something went wrong that should absolutely \
                        never happen! This has been logged, and if it happens again, let the devs know \
                        in the support server.").await;
                        log(ctx, format!("Failed to convert to u64! {}", e));
                        return Ok(());
                    }
                },
                Err(e) => {
                    msg
                    .reply(
                        &ctx.http,
                        "This channel hasn't been set up! Run the `setup` command, with this channel \
                            set as the result channel."
                    )
                    .await;
                    log(ctx, format!("Couldn't get index 1 {:?}", e)).await;
                    return Ok(());
                }
            };
            (token, id)
        }
        Err(e) => {
            msg.reply(
                &ctx.http,
                "This channel hasn't been set up! Run the `setup` command, with this channel \
                            set as the result channel.",
            )
            .await;
            log(ctx, format!("Couldn't get row from table {:?}", e)).await;
            return Ok(());
        }
    };

    let webhook = match ctx.http.http().get_webhook_with_token(id, &*token).await {
        Ok(w) => w,
        Err(e) => {
            msg.reply(
                &ctx.http,
                format!(
                    "A error occurred while fetching the webhook. Make sure it hasn't been \
                    deleted. If it has, re-run `setup`. {}",
                    e
                ),
            )
            .await;
            return Ok(());
        }
    };

    if let Err(e) = msg
        .reply(&ctx.http, &"Initializing voice client, please wait...")
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

            let ctx1 = Arc::new(ctx.clone());

            let receiver = Receiver::new(webhook, ctx1);

            if let Err(e) = handler.mute(true).await {
                if let Err(e) = msg
                    .reply(
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
                .reply(&ctx.http, &format!("Joined {}", connect_to.mention()))
                .await
            {
                log(ctx, format!("Failed to send message! {:?}", e)).await
            }
        }
        Err(e) => {
            if let Err(e) = msg
                .reply(&ctx.http, format!("Error joining the channel: {}", e))
                .await
            {
                log(ctx, format!("Failed to send message! {:?}", e)).await
            }
        }
    }

    Ok(())
}
