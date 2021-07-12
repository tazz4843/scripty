use chrono::Utc;
use std::lazy::SyncOnceCell as OnceCell;

pub static START_TIME: OnceCell<chrono::DateTime<Utc>> = OnceCell::new();
