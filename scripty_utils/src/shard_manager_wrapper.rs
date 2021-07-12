use serenity::client::bridge::gateway::ShardManager;
use serenity::prelude::TypeMapKey;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ShardManagerWrapper;
impl TypeMapKey for ShardManagerWrapper {
    type Value = Arc<RwLock<Arc<serenity::prelude::Mutex<ShardManager>>>>;
}
