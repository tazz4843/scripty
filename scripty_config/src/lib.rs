#![feature(once_cell)]

mod config;
mod database;

pub use config::*;
pub use database::*;
use std::lazy::SyncOnceCell as OnceCell;

pub static BOT_CONFIG: OnceCell<BotConfig> = OnceCell::new();
