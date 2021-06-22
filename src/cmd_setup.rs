use serenity::{
    builder::CreateEmbed,
    client::Context,
    collector::CollectReply,
    framework::standard::{macros::command, CommandResult},
    model::prelude::{Channel, ChannelId, ChannelType, Message},
    prelude::Mentionable,
};
use sqlx::query;

use crate::{bind, globals::PgPoolKey, log, send_embed};
use std::{hint, str::FromStr, sync::Arc};
use tokio::time::Duration;

fn is_number(msg: &Arc<Message>) -> bool {
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
    let db = data.get::<PgPoolKey>().unwrap_or_else(|| unsafe {
        hint::unreachable_unchecked()
        // SAFETY: this should never happen if the DB pool is placed in at client init.
    });
    let guild_id = msg.guild_id;

    let guild_id = match guild_id {
        Some(id) => id,
        None => {
            log(ctx, "msg.guild_id is None for the setup command").await;
            let _ = msg.channel_id.say(
                &ctx,
                "We shouldn't be in DMs. Discord seems to have messed up. This is a extremely rare thing. Go buy a lottery ticket."
            ).await; // we're in DMs, we can't do anything if we can't DM them, so that's why this is `let _`
            return Ok(());
        }
    };

    //////////////////////////////////////////////////////
    // make user agree to ToS + Privacy Policy as a CYA //
    //////////////////////////////////////////////////////
    let mut m = match msg
        .channel_id
        .say(
            &ctx.http,
            "By using Scripty you agree to the privacy policy, found here: https://scripty.imaskeleton.me/privacy_policy . \
    Type `ok` within 5 minutes to continue.",
        )
        .await
    {
        Ok(m) => m,
        Err(e) => {
            if let Err(e) = msg.author.direct_message(&ctx.http, |c| {
                c.content(
                    format!(
                        "I failed to send a message in {}! Make sure I have permissions to send messages. {}",
                        msg.channel_id.mention(),
                        e
                    )
                )
            }).await {
                log(ctx, format!("Failed to DM user! {:?}", e)).await;
            };
            return Ok(());
        }
    };
    if CollectReply::new(&ctx)
        .author_id(msg.author.id)
        .channel_id(msg.channel_id)
        .guild_id(
            msg.guild_id
                .expect("Somehow still ended up in DMs for setup command!"),
        )
        .filter(|msg| msg.content.to_lowercase() == "ok")
        .timeout(Duration::from_secs(300))
        .await
        .is_none()
    {
        let _ = m
            .edit(&ctx, |m| m.content("Timed out. Rerun setup to try again."))
            .await;
        return Ok(());
    }
    drop(m);

    /////////////////////////////////////////
    // get the voice chat ID from the user //
    /////////////////////////////////////////
    let mut m = match msg.channel_id.say(&ctx.http, "Paste the ID of the voice chat you want me to transcript messages from.\n\
    If you don't know how to get the ID, see this picture: https://cdn.discordapp.com/attachments/697540103136084011/816353842823561306/copy_id.png").await {
        Ok(m) => m,
        Err(e) => {
            if let Err(e) = msg.author.direct_message(&ctx.http, |c| {
                c.content(format!("I failed to send a message in {}! Make sure I have permissions to send messages. {}", msg.channel_id.mention(), e))
            }).await {
                log(ctx, format!("Failed to DM user! {:?}", e)).await;
            };
            return Ok(());
        }
    };

    let voice_id = match CollectReply::new(&ctx)
        .author_id(msg.author.id)
        .channel_id(msg.channel_id)
        .guild_id(
            msg.guild_id
                .unwrap_or_else(|| unsafe { hint::unreachable_unchecked() }),
        )
        .filter(is_number)
        .timeout(Duration::from_secs(120))
        .await
    {
        Some(m) => {
            let content = m.content.clone();
            u64::from_str(&*content).expect("Somehow got a invalid ID.")
        }
        None => {
            let _ = m
                .edit(&ctx, |m| m.content("Timed out. Rerun setup to try again."))
                .await;
            return Ok(());
        }
    };
    drop(m);

    ////////////////////////////////////////////////////////
    // get the transcription result channel from the user //
    ////////////////////////////////////////////////////////
    let mut m = match msg
        .channel_id
        .say(
            &ctx.http,
            "Now paste the ID of the channel you want me to send the results of transcriptions to.",
        )
        .await
    {
        Ok(m) => m,
        Err(e) => {
            if let Err(e) = msg
                .author
                .direct_message(&ctx.http, |c| {
                    c.content(format!(
                        "I failed to send a message in {}! Make sure I have permissions to send messages. {}",
                        msg.channel_id.mention(),
                        e
                    ))
                })
                .await
            {
                log(ctx, format!("Failed to DM user! {:?}", e)).await;
            };
            return Ok(());
        }
    };

    let result_id = match CollectReply::new(&ctx)
        .author_id(msg.author.id)
        .channel_id(msg.channel_id)
        .guild_id(msg.guild_id.expect("Shouldn't be in DMs!"))
        .filter(is_number)
        .timeout(Duration::from_secs(120))
        .await
    {
        Some(m) => {
            let content = m.content.clone();
            u64::from_str(&*content).expect("Somehow got a invalid ID.")
        }
        None => {
            let _ = m
                .edit(&ctx, |m| m.content("Timed out. Rerun setup to try again."))
                .await;
            return Ok(());
        }
    };
    drop(m);

    //////////////////////////
    // now verify those IDs //
    //////////////////////////

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
                .say(&ctx, "This isn't a guild channel! Try again.")
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
        "INSERT INTO guilds
              (guild_id, default_bind, output_channel, premium_level)
            VALUES ($1, $2, $3, $4)
              ON CONFLICT (guild_id) DO UPDATE
                SET default_bind = $2, output_channel = $3, premium_level = $4;",
        guild_id.0 as i64,
        voice_id as i64,
        result_id as i64,
        0 as i16
    )
    .execute(db)
    .await
    {
        Err(err) => {
            log(ctx, format!("Couldn't insert to guilds: {}", err)).await;
            embed
                .title("Ugh, I couldn't write that down..")
                .description("I just let my developer know, until then you could just try again");
        }
        _ => {
            match query!(
                "INSERT INTO channels (channel_id, webhook_token, webhook_id)
            VALUES($1, $2, $3) ON CONFLICT (channel_id) DO UPDATE SET webhook_token = $2, webhook_id = $3;",
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
                        .description("Give the bot a few moments to join the VC.\n\n\
                        **PLEASE NOTE!**\n\
                        Accuracy will be *extremely* low, unless you are in a perfectly silent room \
                        with a top-of-the-line mic speaking clearly, as well as being a 18 to 24 year \
                        old American male. The devs are trying their hardest to fix this, but it's not \
                        easy. If you have a spare (Linux) computer with 3TB of storage and a Nvidia \
                        GPU with at least 8GB VRAM that we can borrow, we would love to hear from you. \
                        Please get in touch with 0/0 on the support server: https://discord.gg/zero-zero");
                }
            }
        }
    }

    send_embed(ctx, msg, is_error, embed).await;

    if let Err(e) = bind::bind(ctx, voice_id.into(), result_id.into(), guild_id).await {
        msg.reply(ctx, format!("Connecting to VC failed! `{}`. Wait a few more \
                minutes for the bot to run auto-join, and if it still doesn't join, let the devs know.", e)).await?;
    };

    Ok(())
}
