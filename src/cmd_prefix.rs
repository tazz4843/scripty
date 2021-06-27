use crate::{
    globals::{CmdInfo, PgPoolKey},
    log, send_embed,
};
use serenity::{
    builder::CreateEmbed,
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
};
use sqlx::query;

/// The `prefix` command to set the prefix
/// 1. Gets the database from `ctx.data` using `SqlitePoolKey`
/// 2. Gets the prefix to change to from `args.rest().trim()` (Everything except the
/// command's `prefix`, trimmed out of the whitespaces at the beginning and the end) (Doesn't
/// require a prefix anymore if the argument is `""`)
/// 3. Saves it to the `prefixes table` for that `guild ID`, replacing it if it exists
/// 4. Informs the user that it's done
/// # Errors
/// - Logs and tells the user if the `guild_id` is `None`, meaning it's in DMs somehow
/// - Logs and tells the user if getting the database failed
/// - Tells the user if the prefix is longer than 10 characters
/// - Logs and tells the user if the query failed
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
    let mut is_error = true;

    let data = ctx.data.read().await;
    let db = unsafe { data.get::<PgPoolKey>().unwrap_unchecked() };
    let prefix = args.rest().trim();
    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => {
            log(ctx, "msg.guild_id is None for the prefix command").await;
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
        log(ctx, format!("Couldn't insert to prefixes: {}", err)).await;
        embed
            .title("Ugh, I couldn't write that down..")
            .description("I just let my developer know, until then you could just try again");
    } else {
        is_error = false;
        embed.description(if !prefix.is_empty() {
            format!("Voila! My prefix here is now `{}`", prefix)
        } else {
            "Yay! I don't even need a prefix here anymore".to_string()
        });
    }

    send_embed(ctx, msg, is_error, embed).await;
    Ok(())
}

/// The function to run to get the dynamic prefix
/// # Error
/// The errors here might be quietly ignored, logging it or informing the user isn't a good idea
/// since this check will run for every message sent and we don't know if it's a command or not
/// 1. Returns `None` (doesn't run any command) if:
/// - Getting the guild ID failed (DM messages don't go through this check anyway)
/// - Getting the CmdInfo failed
/// - The message's boundary (up to first `longest command character count + longest prefix
/// character count (10)` characters of the message) doesn't contain a command
/// - Otherwise continues
/// 2. Returns `None` and logs if:
/// - Getting the database from SqlitePoolKey failed
/// - Fetching the row failed
/// - Getting the prefix from the row failed
pub async fn prefix_check(ctx: &Context, msg: &Message) -> Option<String> {
    let guild_id = msg.guild_id?;
    let cmd_info = CmdInfo::get()?;
    let content = msg.content.as_str();

    let mut is_cmd = false;
    for cmd in cmd_info.cmds().iter() {
        if content.contains(cmd) {
            if content.starts_with('.') && cmd_info.custom_cmds().contains(cmd) {
                return Some(".".to_string());
            }
            is_cmd = true;
            break;
        }
    }
    if !is_cmd {
        return None;
    }

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
            log(
                ctx,
                format!(
                    "Couldn't fetch prefix from the database for the prefix check: {:?}",
                    err
                ),
            )
            .await;
            None
        }
        Ok(row) => match row {
            Some(row) => row.prefix,
            None => None,
        },
    }
}
