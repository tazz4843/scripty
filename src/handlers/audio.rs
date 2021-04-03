use crate::{deepspeech::run_stt, utils::DECODE_TYPE};
use redis::{aio::Connection, AsyncCommands};
use serenity::{async_trait, prelude::RwLock};
use songbird::{
    driver::DecodeMode,
    model::{
        id::UserId,
        payload::{ClientConnect, ClientDisconnect, Speaking},
    },
    Event, EventContext, EventHandler as VoiceEventHandler,
};
use std::{collections::HashMap, marker::PhantomData, process::Stdio, sync::Arc};
use tokio::{io::AsyncWriteExt, process::Command};
use uuid::Uuid;

#[derive(Clone)]
pub struct Receiver<'a> {
    ssrc_map: Arc<RwLock<HashMap<u32, UserId>>>,
    audio_buffer: Arc<RwLock<HashMap<u32, Vec<i16>>>>,
    encoded_audio_buffer: Arc<RwLock<HashMap<u32, Vec<u8>>>>,
    redis: Arc<RwLock<Connection>>,
    phantom: PhantomData<&'a ()>,
}

impl Receiver<'a> {
    pub fn new(redis: Arc<RwLock<Connection>>) -> Self {
        // You can manage state here, such as a buffer of audio packet bytes so
        // you can later store them in intervals.
        let ssrc_map = Arc::new(RwLock::new(HashMap::new()));
        let audio_buffer = Arc::new(RwLock::new(HashMap::new()));
        let encoded_audio_buffer = Arc::new(RwLock::new(HashMap::new()));
        Self {
            ssrc_map,
            audio_buffer,
            encoded_audio_buffer,
            redis,
            phantom: PhantomData,
        }
    }
}

#[async_trait]
impl VoiceEventHandler for Receiver<'_> {
    //noinspection SpellCheckingInspection
    #[allow(unused_variables)]
    async fn act<'b>(&'b self, ctx: &EventContext<'_>) -> Option<Event> {
        use songbird::EventContext as Ctx;

        match ctx {
            Ctx::SpeakingStateUpdate(Speaking {
                speaking,
                ssrc,
                user_id,
                ..
            }) => {
                // Discord voice calls use RTP, where every sender uses a randomly allocated
                // *Synchronisation Source* (SSRC) to allow receivers to tell which audio
                // stream a received packet belongs to. As this number is not derived from
                // the sender's user_id, only Discord Voice Gateway messages like this one
                // inform us about which random SSRC a user has been allocated. Future voice
                // packets will contain *only* the SSRC.
                //
                // You can implement logic here so that you can differentiate users'
                // SSRCs and map the SSRC to the User ID and maintain this state.
                // Using this map, you can map the `ssrc` in `voice_packet`
                // to the user ID and handle their audio packets separately.
                println!(
                    "Speaking state update: user {:?} has SSRC {:?}, using {:?}",
                    user_id, ssrc, speaking,
                );
                if let Some(user_id) = user_id {
                    let mut map = self.ssrc_map.write().await;
                    map.insert(*ssrc, *user_id);
                    match DECODE_TYPE {
                        DecodeMode::Decrypt => {
                            let mut audio_buf = self.encoded_audio_buffer.write().await;
                            match audio_buf.get_mut(ssrc) {
                                Some(b) => {
                                    b.clear();
                                }
                                None => {
                                    audio_buf.insert(*ssrc, Vec::new());
                                }
                            }
                        }
                        DecodeMode::Decode => {
                            let mut audio_buf = self.audio_buffer.write().await;
                            match audio_buf.get_mut(ssrc) {
                                Some(b) => {
                                    b.clear();
                                }
                                None => {
                                    audio_buf.insert(*ssrc, Vec::new());
                                }
                            }
                        }
                        _ => {
                            panic!("No supported decode mode found!")
                        }
                    }
                } // otherwise just ignore it since we can't do anything about that
            }
            Ctx::SpeakingUpdate { ssrc, speaking } => {
                // You can implement logic here which reacts to a user starting
                // or stopping speaking.
                let uid: u64 = {
                    let map = self.ssrc_map.read().await;
                    match map.get(ssrc) {
                        Some(u) => u.to_string().parse().unwrap(),
                        None => 0,
                    }
                };
                if !*speaking {
                    match DECODE_TYPE {
                        DecodeMode::Decrypt => {
                            // all of this code reeks of https://www.youtube.com/watch?v=lIFE7h3m40U
                            let audio = {
                                let mut buf = self.encoded_audio_buffer.write().await;
                                match buf.insert(*ssrc, Vec::new()) {
                                    Some(a) => a,
                                    None => {
                                        println!(
                                            "Didn't find a user with SSRC {} in the audio buffers.",
                                            ssrc
                                        );
                                        return None;
                                    }
                                }
                            };
                            let file_id = Uuid::new_v4();
                            let file_path = format!("{}.wav", file_id.as_u128());

                            /*
                            match OpenOptions::new()
                                .write(true)
                                .create(true)
                                .open(format!("{}.pcm", file_id.as_u128()))
                                .await
                            {
                                Ok(mut f) => {
                                    for i in audio {
                                        if let Err(e) = f.write_u8(*i).await {
                                            println!("Failed to write byte to file! {}", e);
                                        };
                                    }
                                }
                                Err(e) => {
                                    println!("Failed to open/create file! {}", e);
                                }
                            };
                            */
                            let args = [
                                "-f",
                                "s16be",
                                "-ar",
                                "8000",
                                "-ac",
                                "1",
                                "-acodec",
                                "pcm_s16le",
                                "-i",
                                "-",
                                &file_path,
                            ];

                            let mut child = match Command::new("ffmpeg")
                                .args(&args)
                                .stdin(Stdio::piped())
                                .stdout(Stdio::null())
                                .stderr(Stdio::inherit())
                                .kill_on_drop(true)
                                .spawn()
                            {
                                Err(e) => {
                                    println!("Failed to spawn FFMPEG!");
                                    return None;
                                }
                                Ok(c) => {
                                    println!("Spawned FFMPEG!");
                                    c
                                }
                            };

                            match child.stdin {
                                Some(ref mut stdin) => {
                                    for i in audio {
                                        if let Err(e) = stdin.write_u8(i).await {
                                            println!("Failed to write byte to FFMPEG stdin! {}", e);
                                        };
                                    }
                                }
                                None => {
                                    println!("Failed to open FFMPEG stdin!");
                                    return None;
                                }
                            };
                            // we now have a file named "{}.wav" where {} is a random UUID as a 128-bit integer.
                            let mut rd = self.redis.write().await;
                            match child.wait().await {
                                Ok(_) => {
                                    match run_stt(file_path.clone()).await {
                                        Ok(r) => {
                                            let _ = rd.set::<u64, String, u64>(uid, r).await;
                                        }
                                        Err(e) => {
                                            println!("Failed to run speech-to-text! {}", e);
                                        }
                                    };
                                }
                                Err(e) => {
                                    println!("FFMPEG failed! {}", e);
                                }
                            };
                            if let Err(e) = tokio::fs::remove_file(&file_path).await {
                                println!("Failed to delete {}! {}", &file_path, e);
                            };
                        }
                        DecodeMode::Decode => {
                            println!("Decode mode is DecodeMode::Decode");
                            unimplemented!();

                            /*
                            let audio = self.encoded_audio_buffer.read().await.clone();

                            match OpenOptions::new()
                                .write(true)
                                .create(true)
                                .open(format!("{}.opus", ssrc))
                                .await
                            {
                                Ok(mut f) => {
                                    for i in audio {
                                        if let Err(e) = f.write_u8(i).await {
                                            println!("Failed to write byte to file! {}", e);
                                        };
                                    }
                                }
                                Err(e) => {
                                    println!("Failed to open/create file! {}", e);
                                }
                            };

                            // we now have a file named "{}.opus" where {} is the user's SSRC.
                            // TODO: read and send to STT API
                            {
                                self.audio_buffer.write().await.clear(); // now to clear it
                            }
                             */
                        }
                        _ => {
                            println!("Decode mode is invalid!");
                        }
                    }
                }
                println!(
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
                // An event which fires for every received audio packet,
                // containing the decoded data.

                let uid: u64 = {
                    // block that will drop lock when exited
                    let map = self.ssrc_map.read().await;
                    match map.get(&packet.ssrc) {
                        Some(u) => u.to_string().parse().unwrap(),
                        None => 0,
                    }
                };
                match audio {
                    Some(audio) => {
                        println!("Decoded audio is not currently supported.");
                        /*
                        let aud = audio.clone();
                        println!("Audio packet: SSRC {}, user ID {}", packet.ssrc, uid);

                        {
                            // get exclusive write access to the audio buffer and write to it
                            let mut buf = self.audio_buffer.write().await;
                            let mut j: u32 = 0;
                            for i in aud {
                                buf.push(i);
                                /*
                                if j % 2 == 0 {
                                    buf.push(i);
                                }
                                 */
                                j += 1;
                            }
                        }
                        let mut f = match tokio::fs::File::open(format!("{}.pcm", packet.ssrc)).await {
                            Ok(f) => f,
                            Err(e) => {
                                println!("Failed to open file! {}", e);
                                return None;
                            }
                        };
                        for i in audio {
                            f.write_i16(*i).await;
                        }

                        tokio::fs::write(format!("{}.pcm", packet.ssrc), &audio).await;
                         */
                    }
                    _ => {
                        let audio_range: &usize = &(packet.payload.len() - payload_end_pad);
                        let range = std::ops::Range {
                            start: payload_offset,
                            end: audio_range,
                        };
                        let mut buf = self.encoded_audio_buffer.write().await;
                        let b = match buf.get_mut(&packet.ssrc) {
                            Some(b) => b,
                            None => {
                                return None;
                            }
                        };
                        let mut counter: i64 = -1;
                        for i in &packet.payload {
                            counter += 1;
                            if (counter <= *payload_offset as i64) | (counter > *audio_range as i64)
                            {
                                continue;
                            } else {
                                b.push(*i)
                            }
                        }
                    }
                }
            }
            Ctx::RtcpPacket {
                packet,
                payload_offset,
                payload_end_pad,
            } => {
                // An event which fires for every received rtcp packet,
                // containing the call statistics and reporting information.
                // Probably ignorable for our purposes.
            }
            Ctx::ClientConnect(ClientConnect {
                audio_ssrc,
                video_ssrc,
                user_id,
                ..
            }) => {
                // You can implement your own logic here to handle a user who has joined the
                // voice channel e.g., allocate structures, map their SSRC to User ID.
                {
                    // block that will drop the lock when exited
                    let mut map = self.ssrc_map.write().await;
                    map.insert(*audio_ssrc, *user_id);
                }
                println!(
                    "Client connected: user {:?} has audio SSRC {:?}, video SSRC {:?}",
                    user_id, audio_ssrc, video_ssrc,
                );
            }
            Ctx::ClientDisconnect(ClientDisconnect { user_id, .. }) => {
                // You can implement your own logic here to handle a user who has left the
                // voice channel e.g., finalise processing of statistics etc.
                // You will typically need to map the User ID to their SSRC; observed when
                // speaking or connecting.
                let key: Option<u32> = {
                    let map = self.ssrc_map.read().await;
                    let mut id: Option<u32> = None;
                    for i in map.iter() {
                        // walk the map to find the UserId
                        if i.1 == user_id {
                            id = Some(*i.0);
                            break;
                        }
                    }
                    id
                };
                if let Some(u) = key {
                    match DECODE_TYPE {
                        DecodeMode::Decrypt => {
                            let mut audio_buf = self.encoded_audio_buffer.write().await;
                            audio_buf.remove(&u);
                        }
                        DecodeMode::Decode => {
                            let mut audio_buf = self.audio_buffer.write().await;
                            audio_buf.remove(&u);
                        }
                        _ => {
                            unimplemented!("No supported decode mode found!")
                        }
                    }
                    let mut map = self.ssrc_map.write().await;
                    map.remove(&u);
                };

                println!("Client disconnected: user {:?}", user_id);
            }
            _ => {
                // We won't be registering this struct for any more event classes.
                unimplemented!()
            }
        }

        None
    }
}
