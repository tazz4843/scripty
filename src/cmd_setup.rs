use serenity::{
    builder::CreateEmbed,
    client::Context,
    collector::CollectReply,
    framework::standard::{macros::command, CommandResult},
    model::{
        channel::{Channel, ChannelType, Message},
        prelude::ChannelId,
    },
    prelude::Mentionable,
};
use sqlx::query;

use crate::{bind, globals::PgPoolKey, log, send_embed};
use std::{str::FromStr, sync::Arc};
use tokio::time::Duration;

fn collector(msg: &Arc<Message>) -> bool {
    u64::from_str(&*msg.content).is_ok()
}

#[command("setup")]
#[aliases("start", "init", "initialize")]
#[required_permissions("MANAGE_GUILD")]
#[only_in("guilds")]
#[bucket = "expensive"]
#[description = "Set up the bot."]
async fn cmd_setup(ctx: &Context, msg: &Message) -> CommandResult {
    let mut embed = CreateEmbed::default();
    let mut is_error = true;

    let data = ctx.data.read().await;
    let db = data.get::<PgPoolKey>();
    let guild_id = msg.guild_id;

    if guild_id.is_none() {
        log(ctx, "msg.guild_id is None for the setup command").await;
        embed
            .title("Something weird happened and I let you use this command in DMs")
            .description("We have to be in a guild to setup the bot, no?");
    };
    if db.is_none() {
        log(
            ctx,
            "Couldn't get SqlitePool for the setup command".to_string(),
        )
        .await;
        embed
            .title("Now this is super weird and scary")
            .description(
                "I lost my whole book where I write things down, sorry..\n
            I let my dev know, until then just please wait.",
            );
    };

    if msg.channel_id.say(&ctx.http, "Paste the ID of the voice chat you want me to transcript messages from.\n\
    If you don't know how to get the ID, see this picture: https://cdn.discordapp.com/attachments/697540103136084011/816353842823561306/copy_id.png").await.is_err() {
        if let Err(e) = msg.author.direct_message(&ctx.http, |c| {
            c.content(format!("I failed to send a message in {}! Make sure I have permissions to send messages.", msg.channel_id.mention()))
        }).await {
            log(ctx, format!("Failed to DM user! {:?}", e)).await;
        };
        return Ok(());
    };
    let voice_id = if let Some(m) = CollectReply::new(&ctx)
        .author_id(msg.author.id)
        .channel_id(msg.channel_id)
        .guild_id(
            msg.guild_id
                .expect("Somehow still ended up in DMs for setup command!"),
        )
        .filter(collector)
        .timeout(Duration::from_secs(120))
        .await
    {
        let content = m.content.clone();
        Some(u64::from_str(&*content).expect("Somehow got a invalid ID."))
    } else {
        embed
            .title("No response!")
            .description("You didn't respond to the voice chat question with a ID in time!");
        None
    };

    if msg
        .channel_id
        .say(
            &ctx.http,
            "Now paste the ID of the channel you want me to send the results of transcriptions to.",
        )
        .await
        .is_err()
    {
        if let Err(e) = msg
            .author
            .direct_message(&ctx.http, |c| {
                c.content(format!(
                "I failed to send a message in {}! Make sure I have permissions to send messages.",
                msg.channel_id.mention()
            ))
            })
            .await
        {
            log(ctx, format!("Failed to DM user! {:?}", e)).await;
        };
        return Ok(());
    };
    let result_id = if let Some(m) = CollectReply::new(&ctx)
        .author_id(msg.author.id)
        .channel_id(msg.channel_id)
        .guild_id(msg.guild_id.expect("Shouldn't be in DMs!"))
        .filter(collector)
        .timeout(Duration::from_secs(120))
        .await
    {
        let content = m.content.clone();
        Some(u64::from_str(&*content).expect("Somehow got a invalid ID."))
    } else {
        embed
            .title("No response!")
            .description("You didn't respond to the result question with a ID in time!");
        None
    };

    if let (Some(guild_id), Some(voice_id), Some(result_id), Some(db)) =
        (guild_id, voice_id, result_id, db)
    {
        let vc_id = ChannelId::from(voice_id);
        let final_id = ChannelId::from(result_id);

        match match vc_id.to_channel_cached(&ctx).await {
            Some(c) => c,
            None => match vc_id.to_channel(&ctx).await {
                Ok(c) => c,
                Err(e) => {
                    msg.channel_id
                        .say(&ctx, format!("I can't convert that to a channel. {:?}", e))
                        .await?;
                    return Ok(());
                }
            },
        } {
            Channel::Guild(c) => match c.kind {
                ChannelType::Voice => {}
                ChannelType::Stage => {}
                _ => {
                    msg.channel_id
                        .say(&ctx, "This isn't a voice channel! Try again.")
                        .await?;
                    return Ok(());
                }
            },
            _ => {
                msg.channel_id
                    .say(&ctx, "This isn't a voice channel! Try again.")
                    .await?;
                return Ok(());
            }
        }

        let (id, token) = match match final_id.to_channel_cached(&ctx).await {
            Some(c) => c,
            None => match final_id.to_channel(&ctx).await {
                Ok(c) => c,
                Err(e) => {
                    msg.channel_id
                        .say(&ctx, format!("I can't convert that to a channel. {:?}", e))
                        .await?;
                    return Ok(());
                }
            },
        } {
            Channel::Guild(c) => match c.kind {
                ChannelType::Text | ChannelType::News => {
                    match c.create_webhook(&ctx, "Scripty Transcriptions").await {
                        Ok(w) => {
                            match w
                                .execute(&ctx, true, |m| {
                                    m.content("Testing transcription webhook...")
                                })
                                .await
                            {
                                Ok(r) => {
                                    if let Some(m) = r {
                                        let _ = m.delete(&ctx).await; // we don't care if deleting failed
                                    }
                                }
                                Err(e) => {
                                    msg.reply(
                                        &ctx,
                                        format!(
                                            "Testing the webhook failed. This \
                                should never happen. Try running the command again. {}",
                                            e
                                        ),
                                    )
                                    .await?;
                                    return Ok(());
                                }
                            }
                            let token = match w.token {
                                Some(t) => t,
                                None => {
                                    msg.channel_id
                                        .say(
                                            &ctx,
                                            "Discord never sent the bot a token for the \
                                            webhook. This should never happen. Try running the command again.",
                                        )
                                        .await?;
                                    return Ok(());
                                }
                            };
                            let webhook_id = w.id;
                            (webhook_id, token)
                        }
                        Err(e) => {
                            msg.channel_id
                                .say(&ctx, format!("I failed to create a webhook for \
                                transcriptions! Make sure I have the Manage Webhooks permission and try again! {}", e))
                                .await?;
                            return Ok(());
                        }
                    }
                }
                _ => {
                    msg.channel_id
                        .say(&ctx, "This isn't a text channel! Try again.")
                        .await?;
                    return Ok(());
                }
            },
            _ => {
                msg.channel_id
                    .say(&ctx, "This isn't a text channel! Try again.")
                    .await?;
                return Ok(());
            }
        };

        match query!(
            "INSERT INTO guilds (guild_id, default_bind, output_channel)
            VALUES ($1, $2, $3) ON CONFLICT (guild_id) DO UPDATE SET default_bind = $2, output_channel = $3;",
            guild_id.0 as i64,
            voice_id as i64,
            result_id as i64,
        )
        .execute(db)
        .await
        {
            Err(err) => {
                log(ctx, format!("Couldn't insert to guilds: {}", err)).await;
                embed
                    .title("Ugh, I couldn't write that down..")
                    .description(
                        "I just let my developer know, until then you could just try again",
                    );
            }
            _ => {
                match query!(
                    "INSERT INTO channels (channel_id, webhook_token, webhook_id)
            VALUES($1, $2, $3) ON CONFLICT DO NOTHING;",
                    result_id as i64,
                    token,
                    i64::from(id)
                )
                .execute(db)
                .await
                {
                    Err(err) => {
                        log(ctx, format!("Couldn't insert to channels: {:?}", err)).await;
                        embed
                            .title("Ugh, I couldn't write that down..")
                            .description(
                                "I just let my developer know, until then you could just try again",
                            );
                    }
                    _ => {
                        is_error = false;
                        embed
                            .title("Set up successfully!")
                            .description("Give the bot a few moments to join the VC.");
                    }
                }
            }
        }
    }

    send_embed(ctx, msg, is_error, embed).await;

    if let (Some(guild_id), Some(voice_id), Some(result_id)) = (guild_id, voice_id, result_id) {
        match bind::bind(ctx, voice_id.into(), result_id.into(), guild_id).await {
            Err(e) => {
                msg.reply(ctx, format!("Connecting to VC failed! `{}`. Wait a few more \
                minutes for the bot to run auto-join, and if it still doesn't join, let the devs know.", e)).await?;
            }
            _ => {}
        };
    }

    Ok(())
}
