use serenity::{
    builder::CreateEmbed,
    client::Context,
    collector::CollectReply,
    framework::standard::{macros::command, CommandResult},
    model::channel::Message,
};
use sqlx::query;

use crate::{globals::SqlitePoolKey, log, send_embed};
use serenity::prelude::Mentionable;
use std::str::FromStr;
use std::sync::Arc;
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
    let db = data.get::<SqlitePoolKey>();
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

    if msg.channel_id.say(&ctx.http, "Paste the ID of the voice chat you want me to transcript messages from.\nIf you don't know how to get the ID, see this picture: https://cdn.discordapp.com/attachments/697540103136084011/816353842823561306/copy_id.png").await.is_err() {
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
        match query(
            "INSERT OR REPLACE INTO guilds (guild_id, default_bind, output_channel)
            VALUES(?, ?, ?);",
        )
        .bind(guild_id.0 as i64)
        .bind(voice_id as i64)
        .bind(result_id as i64)
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
                is_error = false;
                embed.title("Set up successfully!").description(
                    "The bot will join the VC within 10 minutes and stay in there forever.\n
                    Until then there's nothing much to do besides wait.",
                );
            }
        }
    }

    send_embed(ctx, msg, is_error, embed).await;
    Ok(())
}
