use serenity::{
    builder::CreateEmbed,
    client::Context,
    framework::standard::{macros::command, CommandResult},
    model::prelude::{Channel, ChannelId, ChannelType, Message},
};
use sqlx::query;

use crate::msg_handler::handle_message;
use crate::{bind, globals::PgPoolKey, log, send_embed};
use serenity::builder::CreateSelectMenuOption;
use serenity::collector::CollectComponentInteraction;
use serenity::model::prelude::{ButtonStyle, InteractionData, UserId};
use std::hint;
use tokio::time::Duration;

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
    let mut m = match handle_message(&ctx, &msg, |f| {
        f
            .content("By using Scripty you agree to the privacy policy, found here: https://scripty.imaskeleton.me/privacy_policy . \
    Type `ok` within 5 minutes to continue.")
            .components(|c| {
                c.create_action_row(|r| {
                    r.create_button(|b| {
                        b.style(ButtonStyle::Primary)
                            .custom_id("tos_agree")
                            .label("I Agree")
                    })
                })
            })
    }).await
    {
        Some(m) => m,
        None => return Ok(()),
    };
    if CollectComponentInteraction::new(&ctx)
        .author_id(msg.author.id)
        .channel_id(msg.channel_id)
        .guild_id(unsafe { msg.guild_id.unwrap_unchecked() })
        .filter(|action| {
            let data = match action.data {
                Some(ref d) => d,
                None => return false,
            };
            match data {
                InteractionData::ApplicationCommand(_) => false,
                InteractionData::MessageComponent(data) => data.custom_id.as_str() == "tos_agree",
            }
        })
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
    let mut channel_ids = vec![];
    for c in guild_id
        .channels(&ctx)
        .await
        .expect("failed to fetch guild channels")
        .iter()
    {
        match c.1.kind {
            ChannelType::Text | ChannelType::News => {}
            _ => continue,
        };
        if let Ok(perms) =
            c.1.permissions_for_user(&ctx, UserId(ctx.http.application_id))
                .await
        {
            if !perms.manage_webhooks() {
                continue;
            }
        } else {
            continue;
        }

        let mut name = c.1.name.clone();
        name.truncate(25);
        let mut opt = CreateSelectMenuOption::default();
        opt.label(name.clone()).value(c.0 .0);
        channel_ids.push(opt);
    }

    let mut m = match handle_message(&ctx, &msg, |m| {
        m.content("Select the channel you want me to send the results of transcriptions to from the dropdowns below.\n\
        **NOTE**: this only includes channels where i have the Manage Webhooks permission. \
        If the channel you want doesn't show up, give me the Manage Webhooks permission there and try rerunning setup.")
            .components(|c| {
                let (rest, sets) = channel_ids.as_rchunks::<25>();
                let mut i = 0;
                for set in sets {
                    c.create_action_row(|r| {
                        r.create_select_menu(|b| {
                            b.custom_id(format!("result_id_picker_{}", i))
                                .options(|o| {
                                    o.set_options(set.to_vec())
                                })
                        })
                    });
                    i += 1;
                    if i > 5 {break;}
                }
                if i < 5 {
                    c.create_action_row(|r| {
                        r.create_select_menu(|b| {
                            b.custom_id("result_id_picker_overflow")
                                .options(|o| {
                                    o.set_options(rest.to_vec())
                                })
                        })
                    });
                }
                c
            })
    }).await {
        Some(m) => m,
        None => return Ok(()),
    };

    let result_id: u64 = match CollectComponentInteraction::new(&ctx)
        .author_id(msg.author.id)
        .channel_id(msg.channel_id)
        .guild_id(unsafe { msg.guild_id.unwrap_unchecked() })
        .message_id(m.id)
        .collect_limit(1)
        .timeout(Duration::from_secs(60))
        .await
    {
        Some(i) => match i.data {
            Some(ref d) => {
                if let InteractionData::MessageComponent(comp) = d {
                    match comp.values.get(0) {
                        Some(v) => match v.parse() {
                            Ok(v) => v,
                            Err(e) => {
                                let _ = m
                                    .edit(&ctx, |m| {
                                        m.content("Discord had a issue and sent us the wrong ID. Try again.")
                                            .components(|c| c)
                                    })
                                    .await;
                                return Ok(());
                            }
                        },
                        None => {
                            let _ = m
                                .edit(&ctx, |m| {
                                    m.content("Discord had a issue and didn't send us enough IDs. Try again.")
                                        .components(|c| c)
                                })
                                .await;
                            return Ok(());
                        }
                    }
                } else {
                    let _ = m
                        .edit(&ctx, |m| {
                            m.content("Discord had a issue and sent us the wrong type of data. Try again.")
                                .components(|c| c)
                        })
                        .await;
                    return Ok(());
                }
            }
            None => {
                let _ = m
                    .edit(&ctx, |m| {
                        m.content("Discord had a issue and didn't send us any data. Try again.")
                            .components(|c| c)
                    })
                    .await;
                return Ok(());
            }
        },
        None => {
            let _ = m
                .edit(&ctx, |m| {
                    m.content("Timed out. Rerun setup to try again.")
                        .components(|c| c)
                })
                .await;
            return Ok(());
        }
    };

    drop(m);

    ////////////////////////////////////////////////////////
    // get the transcription result channel from the user //
    ////////////////////////////////////////////////////////
    let mut channel_ids = vec![];
    for c in guild_id
        .channels(&ctx)
        .await
        .expect("failed to fetch guild channels")
        .iter()
    {
        match c.1.kind {
            ChannelType::Voice | ChannelType::Stage => {}
            _ => continue,
        };
        if let Ok(perms) =
            c.1.permissions_for_user(&ctx, UserId(ctx.http.application_id))
                .await
        {
            if !perms.connect() {
                continue;
            }
        } else {
            continue;
        }

        let mut name = c.1.name.clone();
        name.truncate(25);
        let mut opt = CreateSelectMenuOption::default();
        opt.label(name.clone()).value(c.0 .0);
        channel_ids.push(opt);
    }

    let mut m = match handle_message(&ctx, &msg, |m| {
        m.content("Select the voice chat you would like me to join and transcript from from the dropdowns below.\n\
        **NOTE**: you can temporarily change this at any time by dragging me to another VC, or permanently by rerunning setup.")
            .components(|c| {
                let (rest, sets) = channel_ids.as_rchunks::<25>();
                let mut i = 0;
                for set in sets {
                    c.create_action_row(|r| {
                        r.create_select_menu(|b| {
                            b.custom_id(format!("result_id_picker_{}", i))
                                .options(|o| {
                                    o.set_options(set.to_vec())
                                })
                        })
                    });
                    i += 1;
                    if i > 5 {break;}
                }
                if i < 5 {
                    c.create_action_row(|r| {
                        r.create_select_menu(|b| {
                            b.custom_id("result_id_picker_overflow")
                                .options(|o| {
                                    o.set_options(rest.to_vec())
                                })
                        })
                    });
                }
                c
            })
    }).await {
        Some(m) => m,
        None => return Ok(()),
    };

    let voice_id: u64 = match CollectComponentInteraction::new(&ctx)
        .author_id(msg.author.id)
        .channel_id(msg.channel_id)
        .guild_id(unsafe { msg.guild_id.unwrap_unchecked() })
        .message_id(m.id)
        .collect_limit(1)
        .timeout(Duration::from_secs(60))
        .await
    {
        Some(i) => match i.data {
            Some(ref d) => {
                if let InteractionData::MessageComponent(comp) = d {
                    match comp.values.get(0) {
                        Some(v) => match v.parse() {
                            Ok(v) => v,
                            Err(e) => {
                                let _ = m
                                    .edit(&ctx, |m| {
                                        m.content("Discord had a issue and sent us the wrong ID. Try again.")
                                            .components(|c| c)
                                    })
                                    .await;
                                return Ok(());
                            }
                        },
                        None => {
                            let _ = m
                                .edit(&ctx, |m| {
                                    m.content("Discord had a issue and didn't send us enough IDs. Try again.")
                                        .components(|c| c)
                                })
                                .await;
                            return Ok(());
                        }
                    }
                } else {
                    let _ = m
                        .edit(&ctx, |m| {
                            m.content("Discord had a issue and sent us the wrong type of data. Try again.")
                                .components(|c| c)
                        })
                        .await;
                    return Ok(());
                }
            }
            None => {
                let _ = m
                    .edit(&ctx, |m| {
                        m.content("Discord had a issue and didn't send us any data. Try again.")
                            .components(|c| c)
                    })
                    .await;
                return Ok(());
            }
        },
        None => {
            let _ = m
                .edit(&ctx, |m| {
                    m.content("Timed out. Rerun setup to try again.")
                        .components(|c| c)
                })
                .await;
            return Ok(());
        }
    };
    drop(m);

    //////////////////////////
    // now verify those IDs //
    //////////////////////////

    let final_id = ChannelId::from(result_id);

    let (id, token) = match match final_id.to_channel(&ctx).await {
        Ok(c) => c,
        Err(e) => {
            msg.channel_id
                .say(&ctx, format!("I can't convert that to a channel. {:?}", e))
                .await?;
            return Ok(());
        }
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
                                    let _ = w.delete_message(&ctx, m.id).await; // we don't care if deleting failed
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
                                            webhook. This should never happen. Try restarting setup.",
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
                    .say(
                        &ctx,
                        "Something weird happened in my code... try restarting setup?",
                    )
                    .await?;
                return Ok(());
            }
        },
        _ => {
            msg.channel_id
                .say(
                    &ctx,
                    "Something weird happened in my code... try restarting setup?",
                )
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
                        Please get in touch with 0/0 on the support server: https://discord.gg/zero-boats");
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
