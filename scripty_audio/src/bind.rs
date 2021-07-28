use super::audio_handler::Receiver;
use scripty_db::PgPoolKey;
use serenity::{
    http::CacheHttp,
    model::prelude::{Channel, ChannelId, ChannelType, GuildId},
    prelude::Context,
};
use songbird::CoreEvent;
use sqlx::query;
use std::{convert::TryInto, sync::Arc};
use tracing::debug;

pub async fn bind(
    ctx: &Context,
    bind_channel: ChannelId,
    transcription_channel: ChannelId,
    guild_id: GuildId,
) -> Result<(), String> {
    let data = ctx.data.read().await;
    let db = data.get::<PgPoolKey>();
    if db.is_none() {
        return Err("No DB pool found.".to_string());
    };

    debug!(guild_id = guild_id.0, "checking channel type");
    match match bind_channel.to_channel(&ctx).await {
        Ok(c) => c,
        Err(e) => {
            return Err(format!("Can't convert to channel: {}", e));
        }
    } {
        Channel::Guild(c) => match c.kind {
            ChannelType::Voice | ChannelType::Stage => {}
            _ => {
                return Err("Not a voice channel.".to_string());
            }
        },
        _ => return Err("Not a guild channel.".to_string()),
    };

    debug!(guild_id = guild_id.0, "checking premium level");
    let premium_level: u8 = match query!(
        "SELECT premium_level FROM guilds WHERE guild_id = $1",
        i64::from(guild_id)
    )
    .fetch_optional(unsafe { db.unwrap_unchecked() })
    .await
    {
        Ok(result) => {
            let result = match result {
                Some(r) => r,
                None => return Err("Guild not found in DB.".to_string()),
            };

            match result.premium_level.try_into() {
                Ok(r) => r,
                Err(e) => return Err(format!("Failed to convert premium level to a u8: {}", e)),
            }
        }
        Err(e) => return Err(format!("DB returned a error: {:?}", e)),
    };

    debug!(
        transcription_id = transcription_channel.0,
        "fetching webhook token/id"
    );
    let (token, id): (String, u64) = match query!(
        "SELECT webhook_token, webhook_id FROM channels WHERE channel_id = $1",
        i64::from(transcription_channel)
    )
    .fetch_optional(unsafe { db.unwrap_unchecked() })
    .await
    {
        Ok(result) => {
            let result = match result {
                Some(r) => r,
                None => return Err("Channel not found in DB.".to_string()),
            };

            let token = match result.webhook_token {
                Some(r) => r,
                None => return Err("Couldn't get webhook token from DB".to_string()),
            };
            let id = match result.webhook_id {
                Some(r) => match r.try_into() {
                    Ok(r) => r,
                    Err(e) => return Err(format!("Couldn't convert webhook ID to i64: {}", e)),
                },
                None => return Err("Couldn't get webhook ID".to_string()),
            };
            (token, id)
        }
        Err(e) => return Err(format!("DB returned a error: {:?}", e)),
    };

    debug!(
        transcription_id = transcription_channel.0,
        "fetching actual webhook"
    );
    let webhook = match ctx.http.http().get_webhook_with_token(id, &*token).await {
        Ok(w) => w,
        Err(e) => return Err(format!("Error while fetching webhook: {}", e)),
    };

    debug!(guild_id = guild_id.0, "loading songbird client");
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    debug!(guild_id = guild_id.0, "connecting to VC");
    let (handler_lock, conn_result) = manager.join(guild_id, bind_channel).await;

    match conn_result {
        Ok(_) => {
            debug!(guild_id = guild_id.0, "connected");
            // NOTE: this skips listening for the actual connection result.
            let mut handler = handler_lock.lock().await;

            let ctx1 = Arc::new(ctx.clone());

            debug!(guild_id = guild_id.0, "creating receiver");
            let receiver =
                Receiver::new(webhook, ctx1, premium_level, guild_id == 675390855716274216).await;

            debug!(guild_id = guild_id.0, "muting self");
            let _ = handler.mute(true).await;

            debug!(guild_id = guild_id.0, "registering receiver");
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
