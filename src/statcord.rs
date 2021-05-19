use crate::globals::{BotConfig, StatcordDataKey};
use serde::{Deserialize, Serialize};
use serenity::client::Context;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

#[allow(dead_code)]
pub struct StatcordData {
    pub servers: AtomicU32,
    pub users: AtomicU32,
    pub commands: AtomicU64,
    pub total_audio_transcript_ms: AtomicU64, // TODO: keep a counter of how many packets have been received, each is 20ms
    pub active_voice_connections: AtomicU32,
}

//noinspection SpellCheckingInspection
#[derive(Serialize, Deserialize)]
pub struct StatcordPostRequest {
    pub id: String,
    pub key: String,
    pub servers: String,
    pub users: String,
    pub active: Vec<String>,
    pub commands: String,
    pub popular: Vec<String>,
    pub memactive: String,
    pub memload: String,
    pub cpuload: String,
    pub bandwidth: String,
    pub custom1: String,
    pub custom2: String,
}

impl StatcordPostRequest {
    fn new(
        id: u64,
        key: String,
        servers: u32,
        users: u32,
        commands: u64,
        custom1: u64,
        custom2: u32,
    ) -> Self {
        Self {
            id: id.to_string(),
            key,
            servers: servers.to_string(),
            users: users.to_string(),
            active: Vec::new(),
            commands: commands.to_string(),
            popular: Vec::new(),
            memactive: "".to_string(),
            memload: "".to_string(),
            cpuload: "".to_string(),
            bandwidth: "".to_string(),
            custom1: custom1.to_string(),
            custom2: custom2.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct StatcordPostResponse {
    error: bool,
    message: String,
    wait: Option<i64>,
}

async fn post_to_statcord(ctx: &Context) {
    let (voice_conns, commands, servers, ms_audio_transcripted, users) = {
        let d = ctx.data.read().await;
        let d = d
            .get::<StatcordDataKey>()
            .expect("statcord data placed in at init");
        if let Some(d) = d {
            let active_voice_conns = d.active_voice_connections.load(Ordering::Relaxed);
            let commands = d.commands.swap(0, Ordering::Relaxed);
            let servers = d.servers.load(Ordering::Relaxed);
            let ms_audio_transcripted = d.total_audio_transcript_ms.swap(0, Ordering::Relaxed);
            let users = d.users.load(Ordering::Relaxed);
            (
                active_voice_conns,
                commands,
                servers,
                ms_audio_transcripted,
                users,
            )
        } else {
            return ();
        }
    };
    let json_body = StatcordPostRequest::new(
        811652199100317726,
        BotConfig::get()
            .expect("bot config inserted at init")
            .statcord_key()
            .clone(),
        servers,
        users,
        commands,
        ms_audio_transcripted,
        voice_conns,
    );
    let client = reqwest::Client::new();
    let res = client
        .post("https://api.statcord.com/v3/stats")
        .json(&json_body);
}
