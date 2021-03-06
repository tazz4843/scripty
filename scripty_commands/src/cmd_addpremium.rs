use scripty_db::PgPoolKey;
use scripty_macros::handle_serenity_error;
use serenity::{
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
};
use sqlx::query;
use std::hint::unreachable_unchecked;

#[command("add_premium")]
#[description = "Adds premium to a guild. arg 1 is the level of premium, arg 2 is the guild ID."]
#[owners_only]
async fn cmd_add_premium(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let success = {
        let data = ctx.data.read().await;
        let db = data
            .get::<PgPoolKey>()
            .unwrap_or_else(|| unsafe { unreachable_unchecked() });
        let level = match args.single::<i16>() {
            Ok(l) => l,
            Err(e) => {
                if let Err(e) = msg
                    .channel_id
                    .send_message(&ctx, |m| {
                        m.content(format!("failed to parse #1 as i64 {}", e))
                    })
                    .await
                {
                    handle_serenity_error!(e);
                }
                return Ok(());
            }
        };
        let guild_id = match args.single::<i64>() {
            Ok(l) => l,
            Err(e) => {
                if let Err(e) = msg
                    .channel_id
                    .send_message(&ctx, |m| {
                        m.content(format!("failed to parse #2 as i64 {}", e))
                    })
                    .await
                {
                    handle_serenity_error!(e);
                }

                return Ok(());
            }
        };
        query!(
            "UPDATE guilds SET premium_level = $1 WHERE guild_id = $2",
            level,
            guild_id
        )
        .execute(db)
        .await
    };
    if let Err(e) = msg
        .channel_id
        .send_message(&ctx, |m| {
            m.content(format!("here's the result: {:?}", success))
        })
        .await
    {
        handle_serenity_error!(e);
    }

    Ok(())
}
