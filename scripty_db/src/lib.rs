#![feature(once_cell)]

mod connect;
pub use connect::*;

use serenity::prelude::TypeMapKey;
use sqlx::{Pool, Postgres};
use std::lazy::SyncOnceCell as OnceCell;

/// This OnceCell contains a PostgreSQL pool that you can use anywhere in the bot.
/// If it isn't populated yet, `self::set_db` has not been called yet. You're too early.
/// You should not populate this manually, and rather rely on `self::set_db` to do so for you,
/// as it handles also creating the tables if need be.
pub static PG_POOL: OnceCell<Pool<Postgres>> = OnceCell::new();

/// A wrapper around a `Pool<Postgres>`, designed to be inserted into the serenity client's
/// TypeMap that way it can be used around commands.
pub struct PgPoolKey;
impl TypeMapKey for PgPoolKey {
    type Value = Pool<Postgres>;
}
