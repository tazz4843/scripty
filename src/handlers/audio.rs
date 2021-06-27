use crate::{
    deepspeech::{load_model, run_stt, Model},
    metrics::Metrics,
};
use serenity::{
    async_trait,
    model::webhook::Webhook,
    prelude::{Context, RwLock},
};
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
    sync::Arc,
};
use tokio::task;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

fn do_check(
    user_id: &UserId,
    active_users: &tokio::sync::RwLockReadGuard<BTreeSet<UserId>>,
) -> bool {
    active_users.get(user_id).is_none()
}

#[derive(Clone)]
pub struct Receiver {
    ssrc_map: Arc<RwLock<HashMap<u32, UserId>>>,
    audio_buffer: Arc<RwLock<HashMap<u32, Vec<i16>>>>,
    active_users: Arc<RwLock<BTreeSet<UserId>>>,
    next_users: Arc<RwLock<BTreeSet<UserId>>>,
    webhook: Arc<Webhook>,
    context: Arc<Context>,
    premium_level: u8,
    max_users: u16, // seriously if it hits 65535 users in a VC wtf
    ds_model: Arc<std::sync::RwLock<Model>>,
}

// next two both forcibly implement the required types for async code
// we need these because `deepspeech::Model` doesn't impl Send + Sync
// because it's got a FFI type inside of it
unsafe impl Send for Receiver {}
unsafe impl Sync for Receiver {}

impl Receiver {
    pub async fn new(webhook: Webhook, context: Arc<Context>, premium_level: u8) -> Self {
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
        }
    }
}

#[async_trait]
impl VoiceEventHandler for Receiver {
    //noinspection SpellCheckingInspection
    #[allow(unused_variables)]
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        use songbird::EventContext as Ctx;
        debug!("act event for guild {:#?}", self.webhook.guild_id);

        match ctx {
            Ctx::SpeakingStateUpdate(Speaking {
                speaking,
                ssrc,
                user_id,
                ..
            }) => {
                debug!(
                    "Speaking state update: user {:?} has SSRC {:?}, using {:?}",
                    user_id, ssrc, speaking,
                );

                if let Some(user_id) = user_id {
                    if !do_check(&user_id, &self.active_users.read().await) {
                        trace!("user failed the checks");
                        return None;
                    }

                    {
                        trace!("locking SSRC map...");
                        let mut map = self.ssrc_map.write().await;
                        map.insert(*ssrc, *user_id);
                        trace!("dropping SSRC map...");
                    }
                    let mut audio_buf = self.audio_buffer.write().await;
                    audio_buf.insert(*ssrc, Vec::new());
                }
            }
            Ctx::SpeakingUpdate { ssrc, speaking } => {
                let uid: u64 = {
                    let map = self.ssrc_map.read().await;
                    match map.get(ssrc) {
                        Some(u) => u.0,
                        None => 0,
                    }
                };
                if !do_check(&UserId(uid), &self.active_users.read().await) {
                    return None;
                };

                if !*speaking {
                    let audio = {
                        let mut buf = self.audio_buffer.write().await;
                        match buf.insert(*ssrc, Vec::new()) {
                            Some(a) => a,
                            None => {
                                warn!(
                                    "Didn't find a user with SSRC {} in the audio buffers.",
                                    ssrc
                                );
                                return None;
                            }
                        }
                    };

                    let u = match self.context.cache.user(uid).await {
                        Some(u) => {
                            if u.bot {
                                return None;
                            }
                            u
                        }
                        None => {
                            return None;
                        }
                    };

                    let webhook = Arc::clone(&self.webhook);
                    let context = Arc::clone(&self.context);
                    let model = Arc::clone(&self.ds_model);

                    task::spawn(async move {
                        match run_stt(audio, model).await {
                            Ok(r) => {
                                if !r.is_empty() {
                                    let profile_picture = match u.avatar {
                                        Some(a) => format!(
                                            "https://cdn.discordapp.com/avatars/{}/{}.png",
                                            u.id, a
                                        ),
                                        None => u.default_avatar_url(),
                                    };
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
                trace!(
                    "Source {} (ID {}) has {} speaking.",
                    ssrc,
                    uid,
                    if *speaking { "started" } else { "stopped" },
                );
            }
            Ctx::VoicePacket {
                audio,
                packet,
                payload_offset,
                payload_end_pad,
            } => {
                {
                    let client_data = self.context.data.read().await;
                    let metrics = client_data.get::<Metrics>().unwrap_or_else(|| unsafe {
                        unreachable_unchecked()
                    });
                    // 20ms audio packet: if it isn't 20 but rather 30 oh well too bad, it's only 10ms we lose
                    // anything else shouldn't ever happen
                    metrics.ms_transcribed.inc_by(20);
                }

                let uid = {
                    let map = self.ssrc_map.read().await;
                    match map.get(&packet.ssrc) {
                        Some(u) => u.clone(),
                        None => UserId(0),
                    }
                };

                if !do_check(&uid, &self.active_users.read().await) {
                    return None;
                };

                if let Some(audio) = audio {
                    let mut buf = self.audio_buffer.write().await;
                    let b = match buf.get_mut(&packet.ssrc) {
                        Some(b) => b,
                        None => {
                            return None;
                        }
                    };
                    b.extend(audio);
                }
            }
            Ctx::ClientConnect(ClientConnect {
                audio_ssrc,
                video_ssrc,
                user_id,
                ..
            }) => {
                {
                    let mut map = self.ssrc_map.write().await;
                    map.insert(*audio_ssrc, *user_id);
                }
                {
                    let mut active_users = self.active_users.write().await;
                    if active_users.len() > self.max_users as usize {
                        let mut next_users = self.next_users.write().await;
                        next_users.insert(*user_id);
                    } else {
                        active_users.insert(*user_id);
                    };
                }
                debug!(
                    "Client connected: user {:?} has audio SSRC {:?}, video SSRC {:?}",
                    user_id, audio_ssrc, video_ssrc,
                );
            }
            Ctx::ClientDisconnect(ClientDisconnect { user_id, .. }) => {
                if let Some(u) = {
                    let map = self.ssrc_map.read().await;
                    let mut id: Option<u32> = None;
                    for i in map.iter() {
                        if i.1 == user_id {
                            id = Some(*i.0);
                            break;
                        }
                    }
                    id
                } {
                    {
                        let mut audio_buf = self.audio_buffer.write().await;
                        audio_buf.remove(&u);
                    }
                    {
                        let mut map = self.ssrc_map.write().await;
                        map.remove(&u);
                    }
                    {
                        let mut active_users = self.active_users.write().await;
                        active_users.remove(user_id);
                        let mut next_users = self.next_users.write().await;
                        if let Some(user) = next_users.pop_first() {
                            active_users.insert(user);
                        };
                    }
                };

                debug!("Client disconnected: user {:?}", user_id);
            }
            _ => {}
        }

        None
    }
}
