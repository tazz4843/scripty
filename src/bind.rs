use crate::globals::SqlitePoolKey;
use crate::handlers::audio::Receiver;
use serenity::http::CacheHttp;
use serenity::model::channel::{Channel, ChannelType};
use serenity::model::id::{ChannelId, GuildId};
use serenity::prelude::Context;
use songbird::CoreEvent;
use sqlx::{query, Row};
use std::convert::TryInto;
use std::hint::unreachable_unchecked;
use std::sync::Arc;

pub async fn bind(
    ctx: &Context,
    bind_channel: ChannelId,
    transcription_channel: ChannelId,
    guild_id: GuildId,
) -> Result<(), String> {
    let data = ctx.data.read().await;
    let db = data.get::<SqlitePoolKey>();
    if db.is_none() {
        return Err("No DB pool found.".to_string());
    };

    match match bind_channel.to_channel_cached(&ctx).await {
        Some(c) => c,
        None => match bind_channel.to_channel(&ctx).await {
            Ok(c) => c,
            Err(e) => {
                return Err(format!("Can't convert to channel: {}", e));
            }
        },
    } {
        Channel::Guild(c) => match c.kind {
            ChannelType::Voice | ChannelType::Stage => {}
            _ => {
                return Err("Not a voice channel.".to_string());
            }
        },
        _ => return Err("Not a guild channel.".to_string()),
    }

    let (token, id): (String, u64) =
        match query("SELECT webhook_token, webhook_id FROM channels WHERE channel_id = ?")
            .bind(i64::from(transcription_channel))
            .fetch_optional(db.unwrap_or_else(|| unsafe {
                unreachable_unchecked() // why? we've already made 100% sure the DB pool exists above.
            }))
            .await
        {
            Ok(result) => {
                let result = match result {
                    Some(r) => r,
                    None => return Err("Channel not found in DB.".to_string()),
                };

                let token = match result.try_get(0) {
                    Ok(r) => r,
                    Err(e) => return Err(format!("Couldn't get webhook token from DB: {:?}", e)),
                };
                let id = match result.try_get::<i64, usize>(1) {
                    Ok(r) => match r.try_into() {
                        Ok(r) => r,
                        Err(e) => return Err(format!("Couldn't convert webhook ID to i64: {}", e)),
                    },
                    Err(e) => return Err(format!("Couldn't get webhook ID: {:?}", e)),
                };
                (token, id)
            }
            Err(e) => return Err(format!("DB returned a error: {:?}", e)),
        };

    let webhook = match ctx.http.http().get_webhook_with_token(id, &*token).await {
        Ok(w) => w,
        Err(e) => return Err(format!("Error while fetching webhook: {}", e)),
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let (handler_lock, conn_result) = manager.join(guild_id, bind_channel).await;

    match conn_result {
        Ok(_) => {
            // NOTE: this skips listening for the actual connection result.
            let mut handler = handler_lock.lock().await;

            let ctx1 = Arc::new(ctx.clone());

            let receiver = Receiver::new(webhook, ctx1);

            let _ = handler.mute(true).await;

            handler.add_global_event(CoreEvent::SpeakingStateUpdate.into(), receiver.clone());
            handler.add_global_event(CoreEvent::SpeakingUpdate.into(), receiver.clone());
            handler.add_global_event(CoreEvent::VoicePacket.into(), receiver.clone());
            handler.add_global_event(CoreEvent::ClientConnect.into(), receiver.clone());
            handler.add_global_event(CoreEvent::ClientDisconnect.into(), receiver.clone());

            Ok(())
        }
        Err(e) => Err(format!("Error joining the channel: {}", e)),
    }
}