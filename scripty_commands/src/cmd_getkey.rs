use scripty_db::PgPoolKey;
use scripty_macros::handle_serenity_error;
use serenity::{
    builder::CreateEmbed,
    client::Context,
    framework::standard::{macros::command, CommandResult},
    model::channel::Message,
};
use sqlx::query;
use std::hint::unreachable_unchecked;

#[command("get_key")]
#[bucket = "expensive"]
#[description = "Get your API key if you are a premium subscriber."]
async fn cmd_getkey(ctx: &Context, msg: &Message) -> CommandResult {
    let is_allowed = {
        let data = ctx.data.read().await;
        let db = data
            .get::<PgPoolKey>()
            .unwrap_or_else(|| unsafe { unreachable_unchecked() });

        match query!(
            "SELECT premium_level FROM users WHERE user_id = $1",
            *msg.author.id.as_u64() as i64
        )
        .fetch_one(db)
        .await
        {
            Ok(val) => {
                if let Some(val) = val.premium_level {
                    val >= 1
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    };
    let mut embed = CreateEmbed::default();
    let mut error = false;
    if !is_allowed {
        embed.title("Only premium subscribers can use this command!");
        embed.description(
            "To get access to Scripty's speech to text API, become a monthly subscriber \
        at https://github.com/sponsors/tazz4843",
        );
    } else {
        let data = ctx.data.read().await;
        let db = data
            .get::<PgPoolKey>()
            .unwrap_or_else(|| unsafe { unreachable_unchecked() });
        match query!(
            "SELECT api_key FROM api_keys WHERE user_id = $1",
            *msg.author.id.as_u64() as i64
        )
        .fetch_optional(db)
        .await
        {
            Ok(val) => {
                let key = match val {
                    Some(k) => k.api_key,
                    None => {
                        let mut key = String::new();
                        for _ in 0..32 {
                            key.push(rand::random::<char>())
                        }
                        if let Err(e) = query!(
                            "INSERT INTO api_keys VALUES ($1, $2)",
                            key,
                            *msg.author.id.as_u64() as i64
                        )
                        .execute(db)
                        .await
                        {
                            embed.title("Error from database");
                            embed.description(format!(
                                "A unknown error happened while trying to query the database: {}",
                                e
                            ));
                            error = true;
                        }
                        key
                    }
                };
                if !error {
                    match msg
                        .author
                        .direct_message(ctx, |m| m.content(format!("Your API key is {}", key)))
                        .await
                    {
                        Err(e) => {
                            embed.title("Couldn't DM you.");
                            embed.description(format!("Make sure you have DMs allowed! {}", e));
                        }
                        Ok(_) => {
                            embed.title("DMed your API key to you!");
                            embed.description(
                                "Docs can be found at https://api.scripty.imaskeleton.me/help",
                            );
                        }
                    };
                }
            }
            Err(e) => {
                embed.title("Error from database");
                embed.description(format!(
                    "A unknown error happened while trying to query the database: {}",
                    e
                ));
            }
        }
    }
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
