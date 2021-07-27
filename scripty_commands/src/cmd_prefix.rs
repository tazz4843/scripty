use ahash::RandomState;
use dashmap::DashMap;
use scripty_db::{PgPoolKey, PG_POOL};
use scripty_macros::handle_serenity_error;
use serenity::model::prelude::GuildId;
use serenity::{
    builder::CreateEmbed,
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    futures::TryStreamExt,
    model::prelude::Message,
};
use sqlx::query;
use std::lazy::SyncOnceCell as OnceCell;
use std::time::Instant;

static PREFIXES: OnceCell<DashMap<GuildId, Option<String>, RandomState>> = OnceCell::new();

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
            format!("Voila! My prefix here is now `{}`", &prefix)
        } else {
            "Yay! I don't even need a prefix here anymore".to_string()
        });

        PREFIXES
            .get_or_init(|| DashMap::with_hasher(RandomState::new()))
            .insert(guild_id, Some(prefix.to_string()));
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
    let st = Instant::now();

    let guild_id = msg.guild_id?;

    if let Some(prefix) = PREFIXES
        .get_or_init(|| DashMap::with_hasher(RandomState::new()))
        .get(&guild_id)
    {
        let et = Instant::now();
        tracing::debug!("prefix fetched in {}ns", et.duration_since(st).as_nanos());
        return prefix.value().clone();
    }

    let data = ctx.data.read().await;
    let db = unsafe { data.get::<PgPoolKey>().unwrap_unchecked() };

    let ret = match query!(
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
            tracing::warn!(
                "Couldn't fetch prefix from the database for the prefix check: {:?}",
                err,
            );
            None
        }
        Ok(row) => {
            let prefix: Option<String> = match row {
                Some(row) => row.prefix,
                None => None,
            };

            unsafe { PREFIXES.get().unwrap_unchecked() }.insert(guild_id, prefix.clone());

            prefix
        }
    };
    let et = Instant::now();
    tracing::debug!("prefix fetched in {}ns", et.duration_since(st).as_nanos());
    ret
}

pub async fn load_prefixes() {
    let st = Instant::now();

    tracing::info!("Initializing prefix cache...");
    let prefixes = PREFIXES.get_or_init(|| DashMap::with_hasher(RandomState::new()));

    let db = unsafe { PG_POOL.get().unwrap_unchecked() };

    tracing::info("Fetching all prefixes from DB...");
    for i in query!("SELECT guild_id, prefix FROM prefixes")
        .fetch(db)
        .try_next()
        .await
    {
        if let Some(row) = i {
            prefixes.insert(GuildId(row.guild_id as u64), row.prefix);
        }
    }

    let et = Instant::now();
    tracing::info!(
        "Loaded prefix map in {}ns",
        et.duration_since(st).as_nanos()
    );
}
