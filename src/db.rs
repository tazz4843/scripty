//noinspection SpellCheckingInspection
use sprattus::*;

use serenity::model::{
    user,
    guild,
    channel
};

pub enum AccessLevel {
    Unknown = 65536,
    Banned = 0,
    Default = 50,
    Moderator = 100,
    Admin = 150,
    Owner = 200,
    BotModerator = 250,
    BotAdmin = 300,
    BotOwner = 350
}


#[derive(ToSql, FromSql, Debug)]
#[sql(table = "users")]
struct DiscordUser {
    #[sql(primary_key)]
    id: u32,
    snowflake: u64,
    access_level: u16,
}


#[derive(Entity, Default)]
pub struct DiscordGuild {
    pub id: u32,
    pub snowflake: u64,
    pub voice_channel: Option<u64>,
    pub script_channel: Option<u64>
}


#[derive(Entity, Default)]
pub struct DiscordVoiceChannel {
    pub id: u32,
    pub snowflake: u64,
    pub script_channel: Option<u64>
}


pub enum DiscordModel {
    User(DiscordUser),
    Guild(DiscordGuild),
    VoiceChannel(DiscordVoiceChannel)
}

pub enum DiscordObject {
    User(user::User),
    Guild(guild::Guild),
    Channel(channel::GuildChannel)
}


pub async fn fetch_from_db(discord_model: DiscordObject, rb: Rbatis) -> Result<DiscordModel> {
    return match discord_model {
        DiscordObject::User(u) => {
            let db_user_query = rb.new_wrapper().
                eq(&"snowflake", u.id);
            let mut db_user = rb.fetch_by_wrapper("", &db_user_query).await;
            Ok(DiscordModel::User())
        },
        DiscordObject::Guild(g) => {
            Ok(DiscordModel::Guild())
        },
        DiscordObject::Channel(c) => {
            if c.kind != channel::ChannelType::Voice {
                Err(db::Error::E(&"DB channels only support voice channels!"))
            };
            Ok(DiscordModel::VoiceChannel())
        }
    }
}
