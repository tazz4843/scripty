use crate::PG_POOL;
use scripty_config::BotConfig;
use sqlx::postgres::PgConnectOptions;
use sqlx::{query, PgPool, Pool, Postgres};

pub async fn set_db() -> Pool<Postgres> {
    let config = BotConfig::get().expect("Couldn't get BOT_CONFIG to get the database file");
    let (db_host, db_port, db_user, db_password, db_db) = config.db_connection();
    let db_conn_options = PgConnectOptions::new();
    let db = PgPool::connect_with(
        db_conn_options
            .host(db_host)
            .username(db_user)
            .port(db_port)
            .database(db_db)
            .application_name("scripty")
            .password(db_password)
            .statement_cache_capacity(1000_usize),
    )
    .await
    .expect("Couldn't connect to DB");

    query!(
        "CREATE TABLE IF NOT EXISTS prefixes (
        guild_id BIGINT PRIMARY KEY,
        prefix TEXT
    )",
    )
    .execute(&db)
    .await
    .expect("Couldn't create the prefix table.");

    query!(
        "CREATE TABLE IF NOT EXISTS guilds (
        guild_id BIGINT PRIMARY KEY,
        default_bind BIGINT,
        output_channel BIGINT,
        premium_level SMALLINT NOT NULL
    )",
    )
    .execute(&db)
    .await
    .expect("Couldn't create the guild table.");

    query!(
        "CREATE TABLE IF NOT EXISTS users (
        user_id BIGINT PRIMARY KEY,
        premium_level SMALLINT,
        premium_count SMALLINT
    )",
    )
    .execute(&db)
    .await
    .expect("Couldn't create the users table.");

    query!(
        "CREATE TABLE IF NOT EXISTS channels (
        channel_id BIGINT PRIMARY KEY,
        webhook_token TEXT,
        webhook_id BIGINT
    )",
    )
    .execute(&db)
    .await
    .expect("Couldn't create the channel table.");

    query!(
        "CREATE TABLE IF NOT EXISTS api_keys (
           api_key TEXT NOT NULL,
           user_id BIGINT
         )"
    )
    .execute(&db)
    .await
    .expect("Couldn't create the API keys table");

    PG_POOL
        .set(db.clone())
        .expect("pool was already set, don't call `set_db` more than once");

    db
}
