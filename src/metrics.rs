/// Code used from sushiibot
/// https://raw.githubusercontent.com/sushiibot/sushii-2/888fbcdaecc0838e5c3735a5aac677a2d327ef10/src/model/metrics.rs
use chrono::naive::NaiveDateTime;
use chrono::offset::Utc;
use prometheus::{IntCounter, IntCounterVec, IntGauge, Opts, Registry};
use prometheus_static_metric::make_static_metric;
use serenity::{async_trait, model::prelude::*, prelude::*};
use std::sync::Arc;

make_static_metric! {
    pub label_enum UserType {
        user,
        other_bot,
        own,
    }

    pub label_enum EventType {
        channel_create,
        channel_delete,
        channel_pins_update,
        channel_update,
        guild_ban_add,
        guild_ban_remove,
        guild_create,
        guild_delete,
        guild_emojis_update,
        guild_integrations_update,
        guild_member_add,
        guild_member_remove,
        guild_member_update,
        guild_members_chunk,
        guild_role_create,
        guild_role_delete,
        guild_role_update,
        guild_unavailable,
        guild_update,
        message_create,
        message_delete,
        message_delete_bulk,
        message_update,
        presence_update,
        presences_replace,
        reaction_add,
        reaction_remove,
        reaction_remove_all,
        ready,
        resumed,
        typing_start,
        user_update,
        voice_state_update,
        voice_server_update,
        webhook_update,
        unknown,
    }

    pub struct MessageCounterVec: IntCounter {
        "user_type" => UserType,
    }

    pub struct EventCounterVec: IntCounter {
        "event_type" => EventType,
    }
}

pub struct Metrics {
    pub registry: Registry,
    pub start_time: NaiveDateTime,
    pub messages: MessageCounterVec,
    pub events: EventCounterVec,
    pub guilds: IntGauge,
    pub members: IntGauge,
    pub ms_transcribed: IntCounter,
    pub voice_connections: IntGauge,
}

#[allow(clippy::new_without_default)]
impl Metrics {
    pub fn new() -> Self {
        let messages_vec =
            IntCounterVec::new(Opts::new("messages", "Received messages"), &["user_type"]).unwrap();
        let messages_static_vec = MessageCounterVec::from(&messages_vec);

        let events_vec =
            IntCounterVec::new(Opts::new("events", "Gateway events"), &["event_type"]).unwrap();
        let events_static_vec = EventCounterVec::from(&events_vec);

        let guilds_gauge = IntGauge::new("guilds", "Current guilds").unwrap();
        let members_gauge = IntGauge::new("members", "Current members").unwrap();

        let ms_transcribed =
            IntCounter::new("audio_transcribed", "Milliseconds of audio transcribed").unwrap();

        let registry = Registry::new_custom(Some("scripty".into()), None).unwrap();
        registry.register(Box::new(messages_vec)).unwrap();
        registry.register(Box::new(events_vec)).unwrap();
        registry.register(Box::new(guilds_gauge.clone())).unwrap();
        registry.register(Box::new(members_gauge.clone())).unwrap();
        registry.register(Box::new(ms_transcribed.clone())).unwrap();

        Self {
            registry,
            start_time: Utc::now().naive_utc(),
            messages: messages_static_vec,
            events: events_static_vec,
            guilds: guilds_gauge,
            members: members_gauge,
            ms_transcribed,
            voice_connections,
        }
    }
}

#[async_trait]
pub trait MetricsAsync {
    // Need our own trait since serenity's RawEventHandler doesn't use references
    async fn raw_event(&self, ctx: &Context, event: &Event);
}

#[async_trait]
impl MetricsAsync for Metrics {
    async fn raw_event(&self, ctx: &Context, event: &Event) {
        match event {
            Event::MessageCreate(MessageCreateEvent { message, .. }) => {
                self.events.message_create.inc();

                // Regular user
                if !message.author.bot {
                    self.messages.user.inc();
                // Own messages
                } else if message.is_own(&ctx.cache).await {
                    self.messages.own.inc();
                // Other bot messages
                } else {
                    self.messages.other_bot.inc();
                }
            }
            Event::ChannelCreate(_) => self.events.channel_create.inc(),
            Event::ChannelDelete(_) => self.events.channel_delete.inc(),
            Event::ChannelPinsUpdate(_) => self.events.channel_pins_update.inc(),
            Event::ChannelUpdate(_) => self.events.channel_update.inc(),
            Event::GuildBanAdd(_) => self.events.guild_ban_add.inc(),
            Event::GuildBanRemove(_) => self.events.guild_ban_remove.inc(),
            Event::GuildCreate(GuildCreateEvent { guild, .. }) => {
                self.events.guild_create.inc();
                self.guilds.inc();

                self.members.add(guild.member_count as i64);
            }
            Event::GuildDelete(_) => {
                self.events.guild_delete.inc();
                self.guilds.dec();

                // self.members stale value,
                // don't have the guild anymore so don't know how many to sub()
            }
            Event::GuildEmojisUpdate(_) => self.events.guild_emojis_update.inc(),
            Event::GuildIntegrationsUpdate(_) => self.events.guild_integrations_update.inc(),
            Event::GuildMemberAdd(_) => {
                self.events.guild_member_add.inc();
                self.members.inc();
            }
            Event::GuildMemberRemove(_) => {
                self.events.guild_member_remove.inc();
                self.members.dec();
            }
            Event::GuildMemberUpdate(_) => self.events.guild_member_update.inc(),
            Event::GuildMembersChunk(_) => self.events.guild_members_chunk.inc(),
            Event::GuildRoleCreate(_) => self.events.guild_role_create.inc(),
            Event::GuildRoleDelete(_) => self.events.guild_role_delete.inc(),
            Event::GuildRoleUpdate(_) => self.events.guild_role_update.inc(),
            Event::GuildUnavailable(_) => self.events.guild_unavailable.inc(),
            Event::GuildUpdate(_) => self.events.guild_update.inc(),
            Event::MessageDelete(_) => self.events.message_delete.inc(),
            Event::MessageDeleteBulk(_) => self.events.message_delete_bulk.inc(),
            Event::MessageUpdate(_) => self.events.message_update.inc(),
            Event::PresenceUpdate(_) => self.events.presence_update.inc(),
            Event::PresencesReplace(_) => self.events.presences_replace.inc(),
            Event::ReactionAdd(_) => self.events.reaction_add.inc(),
            Event::ReactionRemove(_) => self.events.reaction_remove.inc(),
            Event::ReactionRemoveAll(_) => self.events.reaction_remove_all.inc(),
            Event::Ready(_) => self.events.ready.inc(),
            Event::Resumed(_) => self.events.resumed.inc(),
            Event::TypingStart(_) => self.events.typing_start.inc(),
            Event::UserUpdate(_) => self.events.user_update.inc(),
            Event::VoiceStateUpdate(_) => self.events.voice_state_update.inc(),
            Event::VoiceServerUpdate(_) => self.events.voice_server_update.inc(),
            Event::WebhookUpdate(_) => self.events.webhook_update.inc(),
            Event::Unknown(_) => self.events.unknown.inc(),
            _ => {
                tracing::warn!("Unhandled metrics event: {:?}", event);
            }
        }
    }
}

impl TypeMapKey for Metrics {
    type Value = Arc<Metrics>;
}
