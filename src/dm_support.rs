use serenity::model::channel::GuildChannel;
use serenity::model::prelude::{ChannelCategory, ChannelId, Message, UserId, Webhook};
use serenity::prelude::Context;
/// Inspired by https://github.com/DuckHunt-discord/DHV4/blob/master/src/cogs/private_messages_support.py
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use serenity::utils::Color;

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
    /// Either gets or creates a channel for a user
    /// # Returns
    /// Either returns a tuple with 0 being the channel and 1 being whether the channel existed,
    /// or returns a error that means something went wrong.
    async fn get_or_create_channel(
        &self,
        ctx: &Context,
        msg: &Message,
    ) -> Result<(GuildChannel, bool), ()> {
        let channels = match ctx
            .cache
            .guild_channels(self.support_category.guild_id)
            .await
        {
            Some(c) => c.values().cloned().collect::<Vec<_>>(),
            None => return Err(()),
        };

        match channels
            .iter()
            .filter(|i| {
                if let Ok(j) = i64::from_str(i.name.as_str()) {
                    j == *msg.author.id.as_u64() as i64
                } else {
                    false
                }
            })
            .next()
        {
            Some(i) => Ok((i.clone(), true)),
            None => {
                match self
                    .support_category
                    .guild_id
                    .create_channel(&ctx, |c| {
                        c.category(self.support_category.id)
                            .name(msg.author.id.as_u64().to_string())
                    })
                    .await
                {
                    Ok(i) => Ok((i, false)),
                    Err(e) => {
                        error!("failed to create channel for user: {}", e);
                        Err(())
                    }
                }
            }
        }
    }

    async fn get_hook(&self, ctx: &Context, msg: &Message) -> Webhook {
        if let Some(h) = self
            .webhooks
            .read()
            .await
            .get(&ChannelId(msg.author.id.as_u64().clone()))
        {
            h.clone()
        } else {
            // hook not in cache, go get it from discord
            let channel = self
                .get_or_create_channel(ctx, msg)
                .await
                .expect("can't do anything");
            if channel.1 {
                // didn't have to create, so just fetch the hook, otherwise panic
                channel
                    .0
                    .webhooks(&ctx)
                    .await
                    .expect("can't get hooks")
                    .get(0)
                    .expect("no hook found")
                    .clone()
            } else {
                // had to make the channel, so make the hook and return it
                channel
                    .0
                    .create_webhook(&ctx, "DM Support")
                    .await
                    .expect("failed to make hook")
            }
        }
    }

    pub async fn cleanup_old_channels(&self, _ctx: Arc<Context>) {
        // TODO: add a cleanup for old channels: this function is currently a no-op
    }

    pub async fn handle_dm_message(&self, ctx: &Context, message: &Message) {
        if let Some(id) = message.guild_id {
            warn!(
                "got a message in DM support handler for guild {}... ignoring!",
                id
            );
            return;
        }

        if self.blocked_users.read().await.contains(&message.author.id) {
            info!(
                "ignoring {}#{}'s message due to blacklist",
                message.author.name, message.author.id
            );
            return;
        }

        let hook = self.get_hook(ctx, message).await;
        let content = message.content_safe(&ctx).await;

        let _ = hook
            .execute(ctx, true, |m| {
                m.avatar_url(message.author.face())
                    .content(content)
                    .username(format!(
                        "{}#{}",
                        message.author.name, message.author.discriminator
                    ))
            })
            .await;
        ()
    }

    pub async fn handle_support_response(&self, ctx: &Context, msg: &Message) {
        if msg.content.starts_with('>') {
            return;
        }

        match msg.guild_id {
            Some(g) => {
                if g != self.support_category.guild_id {
                    warn!(
                        "got a server message in a server that isn't support server... ignoring!"
                    );
                    return;
                }
            }
            None => {
                warn!("got a DM message in support response handler... ignoring!");
                return;
            }
        }

        let user = match msg.channel_id.name(ctx).await.expect("no channel name?").parse::<u64>() {
            Ok(id) => UserId(id)
                .to_user(ctx)
                .await
                .expect("something went wrong while fetching user"),
            Err(_) => {
                return;
            } // this is expected since the first level handler shouldn't handle this situation
        };


        let content = msg.content_safe(ctx).await;
        if let Err(e) = user.direct_message(ctx, |m| {
            m.embed(|e| {
                e.author(|a| {
                    a.icon_url(msg.author.face())
                        .name(format!("{}#{}", msg.author.name, msg.author.discriminator))
                }).color(Color::BLITZ_BLUE)
                    .description(content)
            })
        }).await {
            msg.channel_id.send_message(ctx, |m| {
                m.content(format!("can't send message to this user for some reason: {}\n\
                consider closing the DM", e))
            }).await.expect("couldn't send messages in the channel, is discord broken?");
        };
    }
}
