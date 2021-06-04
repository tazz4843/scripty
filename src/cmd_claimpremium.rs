use crate::globals::PgPoolKey;
use crate::send_embed;
use serenity::{
    builder::CreateEmbed,
    client::Context,
    framework::standard::{macros::command, CommandResult},
    model::channel::Message,
};
use sqlx::query;
use std::hint::unreachable_unchecked;

#[command("claim_premium")]
#[bucket = "expensive"]
#[description = "Use your premium on the guild this command is run in."]
async fn cmd_claimpremium(ctx: &Context, msg: &Message) -> CommandResult {
    let mut is_error = false;
    let mut embed = CreateEmbed::default();
    let _can_use = {
        let data = ctx.data.read().await;
        let db = data
            .get::<PgPoolKey>()
            .unwrap_or_else(|| unsafe { unreachable_unchecked() });
        match query!(
            "SELECT premium_level, premium_count FROM users WHERE user_id = $1",
            *msg.author.id.as_u64() as i64
        )
        .fetch_optional(db)
        .await
        {
            Ok(v) => match v {
                Some(v) => match v.premium_level {
                    Some(i) => match v.premium_count {
                        Some(j) if ((i * 2) + 1) as i32 > j => true,
                        Some(_) => false,
                        None => false,
                    },
                    None => false,
                },
                None => false,
            },
            Err(e) => {
                is_error = true;
                embed.title("Error while fetching from DB");
                embed.description(format!(
                    "This shouldn't've happened! Please let us know in the support server. {}",
                    e
                ));
                false
            }
        }
    };
    if is_error {}
    send_embed(ctx, msg, is_error, embed).await;
    Ok(())
}
