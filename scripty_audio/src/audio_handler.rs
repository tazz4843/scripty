/*
 * Licensed under the EUPL: see LICENSE.md.
 */

use ahash::RandomState;
use scripty_audio_utils::{load_model, run_stt, Model};
use scripty_metrics::{Metrics, METRICS};
use serenity::builder::ExecuteWebhook;
use serenity::model::prelude::Embed;
use serenity::{async_trait, model::webhook::Webhook, prelude::Context};
use smallvec::SmallVec;
use songbird::{
    model::{
        id::UserId,
        payload::{ClientConnect, ClientDisconnect, Speaking},
    },
    Event, EventContext, EventHandler as VoiceEventHandler,
};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};
use tokio::task;
use tracing::{debug, error, trace, warn};

macro_rules! do_check {
    ($active_users:expr, $user_id:expr) => {
        if !$active_users.read().ok()?.contains($user_id) {
            return None;
        }
    };
}

#[derive(Clone)]
pub struct Receiver {
    ssrc_map: Arc<RwLock<HashMap<u32, UserId, RandomState>>>,
    audio_buffer: Arc<RwLock<HashMap<u32, Vec<i16>, RandomState>>>,
    active_users: Arc<RwLock<HashSet<UserId, RandomState>>>,
    next_users: Arc<RwLock<SmallVec<[UserId; 10]>>>, // 10 should be fine
    webhook: Arc<Webhook>,
    context: Arc<Context>,
    premium_level: u8,
    max_users: u16, // seriously if it hits 65535 users in a VC wtf
    ds_model: Arc<RwLock<Model>>,
    verbose: bool,
}

// next two both forcibly implement the required types for async code
// we need these because `deepspeech::Model` doesn't impl Send + Sync
// because it's got a FFI type inside of it
unsafe impl Send for Receiver {}
unsafe impl Sync for Receiver {}

impl Receiver {
    pub async fn new(
        webhook: Webhook,
        context: Arc<Context>,
        premium_level: u8,
        verbose: bool,
    ) -> Self {
        let max_users = match premium_level {
            0 => 10,
            1 => 25,
            2 => 50,
            3 => 100,
            4 => 250,
            _ => u16::MAX,
        };

        if let Some(id) = webhook.guild_id {
            trace!("constructing new receiver for {}", id);
        } else {
            trace!("constructing new receiver for unknown guild");
        };

        let ssrc_map = Arc::new(RwLock::new(HashMap::with_hasher(ahash::RandomState::new())));
        let audio_buffer = Arc::new(RwLock::new(HashMap::with_hasher(ahash::RandomState::new())));
        let webhook = Arc::new(webhook);
        let active_users = Arc::new(RwLock::new(HashSet::with_hasher(ahash::RandomState::new())));
        let next_users = Arc::new(RwLock::new(SmallVec::new()));
        let ds_model = Arc::new(RwLock::new(load_model()));
        Self {
            ssrc_map,
            audio_buffer,
            active_users,
            next_users,
            webhook,
            context,
            premium_level,
            max_users,
            ds_model,
            verbose,
        }
    }
}

#[async_trait]
impl VoiceEventHandler for Receiver {
    //noinspection SpellCheckingInspection
    #[allow(unused_variables)]
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        debug!("act event for guild {:#?}", self.webhook.guild_id);

        match ctx {
            EventContext::SpeakingStateUpdate(Speaking {
                speaking,
                ssrc,
                user_id: Some(user_id),
                ..
            }) => {
                do_check!(&self.active_users, user_id);

                {
                    let mut ssrc_map = self.ssrc_map.write().ok()?;
                    ssrc_map.insert(*ssrc, *user_id);
                }
                {
                    let mut audio_buffer = self.audio_buffer.write().ok()?;
                    audio_buffer.insert(*ssrc, Vec::new());
                }
            }
            EventContext::SpeakingUpdate { ssrc, speaking } => {
                if !*speaking {
                    let uid = {
                        let ssrc_map = self.ssrc_map.read().ok()?;
                        *(ssrc_map.get(ssrc)?)
                    };
                    do_check!(&self.active_users, &uid);

                    let audio = match self.audio_buffer.write().ok()?.get_mut(ssrc) {
                        Some(a) => {
                            let res = a.clone();
                            a.clear();
                            res
                        }
                        None => return None,
                    };

                    let u = self.context.cache.user(uid.0).await?;
                    if u.bot {
                        return None;
                    }

                    // these might seem weird, but these are required that way we can spawn the
                    // task below and move these variables into it without getting lifetime
                    // errors
                    let webhook = Arc::clone(&self.webhook);
                    let context = Arc::clone(&self.context);
                    let model = Arc::clone(&self.ds_model);
                    let verbose = self.verbose;

                    task::spawn(async move {
                        match run_stt(audio, model).await {
                            Ok(r) => {
                                let mut has_result = false;
                                let mut webhook_execute = ExecuteWebhook::default();
                                if let Some(t) = r.transcripts().first() {
                                    let mut transcription = String::new();
                                    let mut err = false;
                                    let mut audio_length = 0;
                                    let mut audio_start = 0;
                                    let tokens = t.tokens();
                                    let total_tokens = tokens.len() - 1;
                                    for (i, token) in tokens.iter().enumerate() {
                                        match token.text() {
                                            Ok(text) => transcription.push_str(text),
                                            Err(e) => {
                                                warn!(
                                                    "transcription contained invalid UTF-8? {}",
                                                    e
                                                );
                                                if verbose {
                                                    err = true;
                                                } else {
                                                    return;
                                                }
                                            }
                                        };
                                        if verbose {
                                            if i == 0 {
                                                audio_start = token.timestep() * 20
                                            } else if i == total_tokens {
                                                audio_length = token.timestep() * 20
                                            }
                                        }
                                    }

                                    if verbose {
                                        let embed = Embed::fake(|x| {
                                            x.description(format!(
                                                "**Transcription**\n{}\n\n\
                                                    **Confidence %**\n{}\n\n\
                                                    **Start Offset (ms)**\n{}\n\n\
                                                    **Length (ms)**\n{}\n\n\
                                                    **Total Possiblities**\n{}",
                                                transcription,
                                                t.confidence() * 100.0,
                                                audio_start,
                                                audio_length,
                                                r.transcripts().len()
                                            ));
                                            if err {
                                                x.field(
                                                    "Note",
                                                    "UTF-8 decoding error was detected",
                                                    false,
                                                );
                                            }
                                            x
                                        });
                                        webhook_execute.embeds(vec![embed]);
                                    } else {
                                        webhook_execute.content(transcription);
                                    }
                                    has_result = true;
                                } else if verbose {
                                    webhook_execute.content("No transcriptions found");
                                    has_result = true;
                                }

                                if has_result {
                                    webhook_execute.avatar_url(u.face()).username(u.name);

                                    let _ = webhook
                                        .execute(&context, false, |m| {
                                            *m = webhook_execute;
                                            m
                                        })
                                        .await;
                                }
                            }
                            Err(e) => {
                                error!("Failed to run speech-to-text! {}", e);
                            }
                        };
                    });
                }
            }
            EventContext::VoicePacket {
                audio,
                packet,
                payload_offset,
                payload_end_pad,
            } => {
                // this code needs to be insanely optimized
                // so we're trying to do stuff with as little overhead as possible
                let st = std::time::Instant::now();

                do_check!(
                    &self.active_users,
                    self.ssrc_map.read().ok()?.get(&packet.ssrc)?
                );

                if let Some(audio) = audio {
                    if let Some(b) = self.audio_buffer.write().ok()?.get_mut(&packet.ssrc) {
                        b.extend(audio)
                    };
                }

                let et = std::time::Instant::now();
                {
                    let client_data = self.context.data.read().await;
                    let m = unsafe { METRICS.get().unwrap_unchecked() };
                    let metrics = client_data
                        .get::<Metrics>()
                        .unwrap_or_else(|| unsafe { unreachable_unchecked() });
                    // 20ms audio packet: if it isn't 20 but rather 30 oh well too bad, it's only 10ms we lose
                    // anything else shouldn't ever happen
                    metrics.ms_transcribed.inc_by(20);
                    metrics.avg_audio_process_time.set(
                        (et.duration_since(st).as_nanos() as i64
                            + metrics.avg_audio_process_time.get())
                            / 2,
                    );
                }
            }
            EventContext::ClientConnect(ClientConnect {
                audio_ssrc,
                video_ssrc,
                user_id,
                ..
            }) => {
                {
                    let mut ssrc_map = self.ssrc_map.write().ok()?;
                    ssrc_map.insert(*audio_ssrc, *user_id);
                }
                {
                    let mut active_users = self.active_users.write().ok()?;
                    if active_users.len() >= self.max_users as usize {
                        let mut next_users = self.next_users.write().ok()?;
                        next_users.push(*user_id);
                    } else {
                        active_users.insert(*user_id);
                    };
                }
            }
            EventContext::ClientDisconnect(ClientDisconnect { user_id, .. }) => {
                let ssrc_map = self.ssrc_map.read().ok()?;
                if let Some(u) =
                    ssrc_map.iter().find_map(
                        |(ssrc, uid)| {
                            if uid == user_id {
                                Some(*ssrc)
                            } else {
                                None
                            }
                        },
                    )
                {
                    {
                        let mut audio_buffer = self.audio_buffer.write().ok()?;
                        audio_buffer.remove(&u);
                    }
                    {
                        let mut ssrc_map = self.ssrc_map.write().ok()?;
                        ssrc_map.remove(&u);
                    }
                    {
                        let mut active_users = self.active_users.write().ok()?;
                        active_users.remove(user_id);
                        let mut next_users = self.next_users.write().ok()?;
                        if let Some(user) = next_users.pop() {
                            active_users.insert(user);
                        };
                    }
                };
            }
            _ => {}
        }

        None
    }
}
