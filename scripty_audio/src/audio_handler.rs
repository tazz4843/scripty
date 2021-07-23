use dashmap::{DashMap, DashSet};
use scripty_audio_utils::{load_model, run_stt, Model};
use scripty_metrics::Metrics;
use serenity::{async_trait, model::webhook::Webhook, prelude::Context};
use songbird::{
    model::{
        id::UserId,
        payload::{ClientConnect, ClientDisconnect, Speaking},
    },
    Event, EventContext, EventHandler as VoiceEventHandler,
};
use std::{
    collections::{BTreeSet, HashMap},
    hint::unreachable_unchecked,
    sync::{Arc, RwLock},
};
use tokio::task;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

fn do_check(user_id: &UserId, active_users: &std::sync::RwLockReadGuard<BTreeSet<UserId>>) -> bool {
    active_users.get(user_id).is_none()
}

#[derive(Clone)]
pub struct Receiver {
    ssrc_map: Arc<DashMap<u32, UserId>>,
    audio_buffer: Arc<RwLock<HashMap<u32, Vec<i16>>>>,
    active_users: Arc<RwLock<BTreeSet<UserId>>>,
    next_users: Arc<RwLock<BTreeSet<UserId>>>,
    webhook: Arc<Webhook>,
    context: Arc<Context>,
    premium_level: u8,
    max_users: u16, // seriously if it hits 65535 users in a VC wtf
    ds_model: Arc<std::sync::RwLock<Model>>,
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

        let ssrc_map = Arc::new(RwLock::new(HashMap::new()));
        let audio_buffer = Arc::new(RwLock::new(HashMap::new()));
        let webhook = Arc::new(webhook);
        let active_users = Arc::new(RwLock::new(BTreeSet::new()));
        let next_users = Arc::new(RwLock::new(BTreeSet::new()));
        let ds_model = Arc::new(std::sync::RwLock::new(load_model()));
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
                if !do_check(
                    user_id,
                    &self
                        .active_users
                        .read()
                        .expect("thread panicked while holding active user lock"),
                ) {
                    return None;
                }

                self.ssrc_map.insert(*ssrc, *user_id);
                let mut audio_buf = self
                    .audio_buffer
                    .write()
                    .expect("thread panicked while holding audio buffer lock");
                audio_buf.insert(*ssrc, Vec::new());
            }
            EventContext::SpeakingUpdate { ssrc, speaking } => {
                let uid: u64 = match self.ssrc_map.get(ssrc) {
                    Some(u) => u.0,
                    None => 0,
                };
                if !do_check(
                    &UserId(uid),
                    &self
                        .active_users
                        .read()
                        .expect("thread panicked while holding active user lock"),
                ) {
                    return None;
                };

                if !*speaking {
                    let audio = {
                        let mut buf = self
                            .audio_buffer
                            .write()
                            .expect("thread panicked while holding audio buffer lock");
                        match buf.get_mut(ssrc) {
                            Some(a) => {
                                let res = a.clone();
                                a.clear();
                                res
                            }
                            None => return None,
                        }
                    };

                    let u = match self.context.cache.user(uid).await {
                        Some(u) => {
                            if u.bot {
                                return None;
                            }
                            u
                        }
                        None => return None,
                    };

                    let webhook = Arc::clone(&self.webhook);
                    let context = Arc::clone(&self.context);
                    let model = Arc::clone(&self.ds_model);

                    task::spawn(async move {
                        match run_stt(audio, model).await {
                            Ok(r) => {
                                if !r.is_empty() {
                                    let profile_picture = u.face();
                                    let name = u.name;

                                    let _ = webhook
                                        .execute(&context, false, |m| {
                                            m.avatar_url(profile_picture).content(r).username(name)
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
                {
                    let client_data = self.context.data.read().await;
                    let metrics = client_data
                        .get::<Metrics>()
                        .unwrap_or_else(|| unsafe { unreachable_unchecked() });
                    // 20ms audio packet: if it isn't 20 but rather 30 oh well too bad, it's only 10ms we lose
                    // anything else shouldn't ever happen
                    metrics.ms_transcribed.inc_by(20);
                }

                let uid = match self.ssrc_map.get(&packet.ssrc) {
                    Some(u) => *u,
                    None => return None,
                };

                if !do_check(
                    &uid,
                    &self
                        .active_users
                        .read()
                        .expect("thread panicked while holding active user lock"),
                ) {
                    return None;
                };

                if let Some(audio) = audio {
                    let mut buf = self
                        .audio_buffer
                        .write()
                        .expect("thread panicked while holding audio buffer lock");
                    let b = match buf.get_mut(&packet.ssrc) {
                        Some(b) => b,
                        None => return None,
                    };
                    b.extend(audio);
                }
            }
            EventContext::ClientConnect(ClientConnect {
                audio_ssrc,
                video_ssrc,
                user_id,
                ..
            }) => {
                self.ssrc_map.insert(*audio_ssrc, *user_id);
                {
                    let mut active_users = self
                        .active_users
                        .write()
                        .expect("thread panicked while holding active user lock");
                    if active_users.len() >= self.max_users as usize {
                        let mut next_users = self
                            .next_users
                            .write()
                            .expect("thread panicked while holding next user lock");
                        next_users.insert(*user_id);
                    } else {
                        active_users.insert(*user_id);
                    };
                }
            }
            EventContext::ClientDisconnect(ClientDisconnect { user_id, .. }) => {
                if let Some(u) = self.ssrc_map.iter().find_map(|i| {
                    if i.value() == user_id {
                        Some(*i.key())
                    } else {
                        None
                    }
                }) {
                    {
                        let mut audio_buf = self
                            .audio_buffer
                            .write()
                            .expect("thread panicked while holding audio buffer lock");
                        audio_buf.remove(&u);
                    }
                    self.ssrc_map.remove(&u);
                    {
                        let mut active_users = self
                            .active_users
                            .write()
                            .expect("thread panicked while holding active user lock");
                        active_users.remove(user_id);
                        let mut next_users = self
                            .next_users
                            .write()
                            .expect("thread panicked while holding next user lock");
                        if let Some(user) = next_users.pop_first() {
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
