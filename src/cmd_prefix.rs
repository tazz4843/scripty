use serenity::{
    builder::CreateEmbed,
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
};
use sqlx::{query, Row};

use crate::{
    globals::{CmdInfo, SqlitePoolKey},
    log, send_embed,
};

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
    let db = data.get::<SqlitePoolKey>();
    let prefix = args.rest().trim();
    let guild_id = msg.guild_id;

    if let None = guild_id {
        log(ctx, "msg.guild_id is None for the prefix command").await;
        embed
            .title("Something weird happened and I let you use this command in DMs")
            .description("We have to be in a guild to set the prefix for a guild, no?");
    };
    if let None = db {
        log(
            ctx,
            format!("Couldn't get SqlitePool for the prefix command"),
        )
        .await;
        embed
            .title("Now this is super weird and scary")
            .description("I lost my whole book where I write things down, sorry..");
    };

    if let (Some(guild_id), Some(db)) = (guild_id, db) {
        if prefix.chars().count() > 10 {
            embed
                .title("Your prefix can't be longer than 10 characters")
                .description("Why would you want it that long anyway..");
        } else {
            if let Err(err) = query(
                "INSERT OR REPLACE INTO prefixes (guild_id, prefix)
                VALUES(?, ?);",
            )
            .bind(guild_id.0 as i64)
            .bind(prefix)
            .execute(db)
            .await
            {
                log(ctx, format!("Couldn't insert to prefixes: {}", err)).await;
                embed
                    .title("Ugh, I couldn't write that down..")
                    .description(
                        "I just let my developer know, until then you could just try again",
                    );
            } else {
                is_error = false;
                embed.description(if prefix != "" {
                    format!("Voila! My prefix here is now `{}`", prefix)
                } else {
                    "Yay! I don't even need a prefix here anymore".to_string()
                });
            }
        }
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
            if content.starts_with(".") && cmd_info.custom_cmds().contains(cmd) {
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
    let db = match data.get::<SqlitePoolKey>() {
        Some(db) => db,
        None => {
            log(ctx, "Couldn't get the database for the prefix check").await;
            return None;
        }
    };

    match query("SELECT prefix FROM prefixes WHERE guild_id = ?")
        .bind(guild_id.0 as i64)
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
        Ok(row) => match row?.try_get(0) {
            Ok(prefix) => prefix,
            Err(err) => {
                log(
                    ctx,
                    format!("Couldn't get the prefix column for the guild: {:?}", err),
                )
                .await;
                None
            }
        },
    }
}
