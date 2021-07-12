use scripty_db::PgPoolKey;
use scripty_macros::handle_serenity_error;
use serenity::{
    builder::CreateEmbed,
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
};
use sqlx::query;

#[command("prefix")]
#[aliases(
    "setprefix",
    "set_prefix",
    "set-prefix",
    "changeprefix",
    "change_prefix",
    "change-prefix"
)]
#[required_permissions("MANAGE_GUILD")]
#[only_in("guilds")]
#[bucket = "expensive"]
#[description = "Change the prefix I'll use in this server\n(It can't end with a space though)"]
#[usage = "[your prefix]"]
#[example = "."]
async fn cmd_prefix(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut embed = CreateEmbed::default();

    let data = ctx.data.read().await;
    let db = unsafe { data.get::<PgPoolKey>().unwrap_unchecked() };
    let prefix = args.rest().trim();
    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => {
            tracing::info!("msg.guild_id is None for the prefix command");
            embed
                .title("Something weird happened and I let you use this command in DMs")
                .description("We have to be in a guild to set the prefix for a guild, no?");
            return Ok(());
        }
    };

    if prefix.chars().count() > 10 {
        embed
            .title("Your prefix can't be longer than 10 characters")
            .description("Why would you want it that long anyway..");
    } else if let Err(err) = query!(
        "INSERT INTO prefixes
                 (guild_id, prefix)
             VALUES
                 ($1, $2)
             ON CONFLICT
                 (guild_id)
             DO UPDATE SET
                 prefix = $2;",
        guild_id.0 as i64,
        prefix,
    )
    .execute(db)
    .await
    {
        tracing::info!("Couldn't insert to prefixes: {}", err);
        embed
            .title("Ugh, I couldn't write that down..")
            .description("I just let my developer know, until then you could just try again");
    } else {
        embed.description(if !prefix.is_empty() {
            format!("Voila! My prefix here is now `{}`", prefix)
        } else {
            "Yay! I don't even need a prefix here anymore".to_string()
        });
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

pub async fn prefix_check(ctx: &Context, msg: &Message) -> Option<String> {
    let guild_id = msg.guild_id?;

    let data = ctx.data.read().await;
    let db = unsafe { data.get::<PgPoolKey>().unwrap_unchecked() };

    match query!(
        "SELECT
           prefix
         FROM
           prefixes
         WHERE
           guild_id = $1",
        guild_id.0 as i64
    )
    .fetch_optional(db)
    .await
    {
        Err(err) => {
            tracing::info!(
                "Couldn't fetch prefix from the database for the prefix check: {:?}",
                err,
            );
            None
        }
        Ok(row) => match row {
            Some(row) => row.prefix,
            None => None,
        },
    }
}
