use serenity::model::prelude::{
    ChannelCategory, ChannelId , Message, UserId, Webhook,
};
use serenity::prelude::Context;
/// Inspired by https://github.com/DuckHunt-discord/DHV4/blob/master/src/cogs/private_messages_support.py
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, warn, info};

pub static SUPPORT_OPEN_MESSAGE: &'static str =
    "Welcome to Scripty DM support.\n\
    Messages here are relayed to a select group of volunteers and bot moderators to help you use the bot.\
    For general questions, we also have a support server \
    [here](https://scripty.imaskeleton.me/support).\n
    If you opened the ticket by mistake, just say `close` and we'll close it for you, otherwise, we'll get \
    back to you in a few minutes.";

struct DmSupportInfo {
    support_category: ChannelCategory,
    webhooks: Arc<RwLock<HashMap<ChannelId, Webhook>>>,
    users: Arc<RwLock<HashMap<u64, UserId>>>,
    blocked_users: Arc<RwLock<HashSet<UserId>>>,
}

impl DmSupportInfo {
    async fn cleanup_old_channels(&self, _ctx: Arc<Context>) {
        // TODO: add a cleanup for old channels: this function is currently a no-op
    }

    async fn handle_dm_message(&self, ctx: &Context, message: &Message) {
        if let Some(id) = message.guild_id {
            warn!(
                "got a message in DM support handler for guild {}... ignoring!",
                id
            );
            return;
        }

        if self.blocked_users.read().await.contains(&message.author.id) {
            info!("ignoring {}#{}'s message due to blacklist", message.author.name, message.author.id);
            return;
        }

        let (channel, existed) = {
            let channels = match ctx
                .cache
                .guild_channels(self.support_category.guild_id)
                .await
            {
                Some(c) => c.values().cloned().collect::<Vec<_>>(),
                None => return,
            };

            match channels
                .iter()
                .filter(|i| {
                    if let Ok(j) = i64::from_str(i.name.as_str()) {
                        j == *message.author.id.as_u64() as i64
                    } else {
                        false
                    }
                })
                .next()
            {
                Some(i) => (i.clone(), true),
                None => {
                    match self
                        .support_category
                        .guild_id
                        .create_channel(&ctx, |c| {
                            c.category(self.support_category.id)
                                .name(message.author.id.as_u64().to_string())
                        })
                        .await
                    {
                        Ok(i) => (i, false),
                        Err(e) => {
                            error!("failed to create channel for user: {}", e);
                            return;
                        }
                    }
                }
            }
        };

        if existed {
            // followup message, use the webhook that should've been created
            let hooks = self.webhooks.read().await;
        }
    }
}
