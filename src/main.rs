use serenity::{async_trait, client::{Client, Context, EventHandler}, framework::{
    standard::{
        Args,
        CommandResult, macros::{command, group},
    },
    StandardFramework,
}, model::{
    channel::Message,
    gateway::Ready,
    id::ChannelId,
    misc::Mentionable
}, Result as SerenityResult};
use songbird::{
    CoreEvent,
    driver::{Config as DriverConfig, DecodeMode},
    Event,
    EventContext,
    EventHandler as VoiceEventHandler,
    model::{id::*, payload::{ClientConnect, ClientDisconnect, Speaking}},
    SerenityInit,
    Songbird,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    time::Duration
};
use tokio::{
    fs::File,
    io::AsyncReadExt,
    sync::RwLock
};
use std::sync::Arc;

thread_local!(static SSRC_MAP: RefCell<HashMap<u32, UserId>> = RefCell::new(HashMap::new()));
thread_local!(static AUDIO_DATA: RefCell<HashMap<UserId, Vec<HashMap<Duration, Vec<u16>>>>> = RefCell::new(HashMap::new()));

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

struct Receiver {
    ssrc_map: Arc<RwLock<HashMap<u32, UserId>>>
}

impl Receiver {
    pub fn new() -> Self {
        // You can manage state here, such as a buffer of audio packet bytes so
        // you can later store them in intervals.
        let ssrc_map = Arc::new(RwLock::new(HashMap::new()));
        Self { ssrc_map }
    }
}

#[async_trait]
impl VoiceEventHandler for Receiver {
    #[allow(unused_variables)]
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        use EventContext as Ctx;

        match ctx {
            Ctx::SpeakingStateUpdate(
                Speaking {speaking, ssrc, user_id, ..}
            ) => {
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
                    user_id,
                    ssrc,
                    speaking,
                );
                { // block that will drop the lock when exited
                    let mut map = self.ssrc_map.write().await;
                    map.insert(*ssrc, user_id.unwrap().into());
                }
            },
            Ctx::SpeakingUpdate {ssrc, speaking} => {
                // You can implement logic here which reacts to a user starting
                // or stopping speaking.
                let user_id: Option<&UserId> = None;
                { // block that will drop lock when exited
                    let map = self.ssrc_map.read().await;
                    let user_id = map.get(&ssrc);
                }
                let uid: u64 = match user_id {
                    Some(u) => {
                        u.to_string().parse().unwrap()
                    }
                    None => {
                        0
                    }
                };
                println!(
                    "Source {} (ID {}) has {} speaking.",
                    ssrc,
                    uid,
                    if *speaking {"started"} else {"stopped"},
                );
            },
            Ctx::VoicePacket {audio, packet, payload_offset, payload_end_pad} => {
                // An event which fires for every received audio packet,
                // containing the decoded data.

                let uid: u64 = { // block that will drop lock when exited
                    let map = self.ssrc_map.read().await;
                    match map.get(&packet.ssrc) {
                        Some(u) => {
                            u.to_string().parse().unwrap()
                        }
                        None => {
                            0
                        }
                    }
                };
                if let Some(audio) = audio {
                    println!("Audio packet's first 5 samples: {:?}", audio.get(..5.min(audio.len())));
                    println!(
                        "Audio packet sequence {:05} has {:04} bytes (decompressed from {}), SSRC {}, user ID {}",
                        packet.sequence.0,
                        audio.len() * std::mem::size_of::<i16>(),
                        packet.payload.len(),
                        packet.ssrc,
                        uid
                    );
                } else {
                    println!("RTP packet, but no audio. Driver may not be configured to decode.");
                    let mut audio: Vec<u8> = std::vec::Vec::new();
                    let audio_range: &usize = &(packet.payload.len()-payload_end_pad);
                    println!("Audio range is {}", &audio_range);
                    let range = std::ops::Range {
                        start: payload_offset,
                        end: audio_range,
                    };
                    let mut counter: i64 = -1;
                    for i in &packet.payload {
                        counter += 1;
                        if counter <= *payload_offset as i64 {
                            continue;
                        } else if counter > *audio_range as i64 {
                            continue;
                        } else {
                            audio.extend(vec![i])
                        }
                    }
                    println!("Audio packet sequence {:05} has {:04} bytes. SSRC {}, user ID {}",
                             packet.sequence.0,
                             audio.len() * std::mem::size_of::<u8>(),
                             packet.ssrc,
                             uid
                    );
                    println!("Raw audio data is {:?}", audio)
                }
            },
            Ctx::RtcpPacket {packet, payload_offset, payload_end_pad} => {
                // An event which fires for every received rtcp packet,
                // containing the call statistics and reporting information.
                // Probably ignorable for our purposes.
                println!("RTCP packet received: {:?}", packet);
            },
            Ctx::ClientConnect(
                ClientConnect {audio_ssrc, video_ssrc, user_id, ..}
            ) => {
                // You can implement your own logic here to handle a user who has joined the
                // voice channel e.g., allocate structures, map their SSRC to User ID.
                { // block that will drop the lock when exited
                    let mut map = self.ssrc_map.write().await;
                    map.insert(*audio_ssrc, *user_id);
                }
                println!(
                    "Client connected: user {:?} has audio SSRC {:?}, video SSRC {:?}",
                    user_id,
                    audio_ssrc,
                    video_ssrc,
                );
            },
            Ctx::ClientDisconnect(
                ClientDisconnect {user_id, ..}
            ) => {
                // You can implement your own logic here to handle a user who has left the
                // voice channel e.g., finalise processing of statistics etc.
                // You will typically need to map the User ID to their SSRC; observed when
                // speaking or connecting.
                let mut key: Option<u32> = None;
                {
                    let map = self.ssrc_map.read().await;
                    for i in map.iter() { // walk the map to find the UserId
                        if i.1 == user_id {
                            key = Some(*i.0);
                            break;
                        };
                    }
                }
                match key {
                    Some(u) => {
                        {
                            let mut map = self.ssrc_map.write().await;
                            map.remove(&u);
                        }
                        println!("Removed {} from the user ID map.", u);
                    },
                    None => {
                        println!("Found no user with ID {} in the user ID map.", user_id);
                    }
                };

                println!("Client disconnected: user {:?}", user_id);
            },
            _ => {
                // We won't be registering this struct for any more event classes.
                unimplemented!()
            }
        }

        None
    }
}

#[group]
#[commands(join, leave, ping)]
struct General;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let path = std::env::current_dir().expect("Error while getting CWD!");
    println!("Make sure your config files are in {}", path.display());

    println!("Loading config...");
    let f = File::open("config.json").await;
    let config = match f {
        Ok(mut file) => {
            let mut buf = vec![0; file.metadata().await.unwrap().len() as usize];
            match file.read_exact(&mut buf).await {
                Ok(_) => {
                    println!("Read config file, loading into memory now.")
                }
                Err(e) => {
                    panic!("Failed to read from config file! {}", e)
                }
            }
            let s = std::str::from_utf8(&buf).unwrap();
            println!("Parsing JSON...");
            let x = match json::parse(&s) {
                Ok(c) => {
                    c
                }
                Err(e) => {
                    panic!("Failed to parse JSON! {}", e)
                }
            };
            println!("Loaded config!");
            x
        }
        Err(e) => {
            panic!("Error encountered while opening config.json: {}", e);
        }
    };

    let token = config["token"].as_str().unwrap();

    let framework = StandardFramework::new()
        .configure(|c| c
            .prefix("~"))
        .group(&GENERAL_GROUP);
    // TODO: .on_dispatch_error(fn);

    // Here, we need to configure Songbird to decode all incoming voice packets.
    // If you want, you can do this on a per-call basis---here, we need it to
    // read the audio data that other people are sending us!
    let songbird = Songbird::serenity();
    songbird.set_config(
        DriverConfig::default()
            .decode_mode(DecodeMode::Decrypt)
    );

    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird_with(songbird.into())
        .await
        .expect("Err creating client");

    let _ = client.start().await.map_err(|why| println!("Client ended: {:?}", why));
}


#[command]
#[only_in(guilds)]
async fn join(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let connect_to = match args.single::<u64>() {
        Ok(id) => ChannelId(id),
        Err(_) => {
            check_msg(msg.reply(ctx, "Requires a valid voice channel ID be given").await);

            return Ok(());
        },
    };

    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at initialisation.").clone();

    let (handler_lock, conn_result) = manager.join(guild_id, connect_to).await;

    if let Ok(_) = conn_result {
        // NOTE: this skips listening for the actual connection result.
        let mut handler = handler_lock.lock().await;

        handler.add_global_event(
            CoreEvent::SpeakingStateUpdate.into(),
            Receiver::new(),
        );

        handler.add_global_event(
            CoreEvent::SpeakingUpdate.into(),
            Receiver::new(),
        );

        handler.add_global_event(
            CoreEvent::VoicePacket.into(),
            Receiver::new(),
        );

        handler.add_global_event(
            CoreEvent::RtcpPacket.into(),
            Receiver::new(),
        );

        handler.add_global_event(
            CoreEvent::ClientConnect.into(),
            Receiver::new(),
        );

        handler.add_global_event(
            CoreEvent::ClientDisconnect.into(),
            Receiver::new(),
        );

        check_msg(msg.channel_id.say(&ctx.http, &format!("Joined {}", connect_to.mention())).await);
    } else {
        check_msg(msg.channel_id.say(&ctx.http, "Error joining the channel").await);
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at initialisation.").clone();
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Err(e) = manager.remove(guild_id).await {
            check_msg(msg.channel_id.say(&ctx.http, format!("Failed: {:?}", e)).await);
        }

        check_msg(msg.channel_id.say(&ctx.http,"Left voice channel").await);
    } else {
        check_msg(msg.reply(ctx, "Not in a voice channel").await);
    }

    Ok(())
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    check_msg(msg.channel_id.say(&ctx.http,"Pong!").await);

    Ok(())
}

/// Checks that a message successfully sent; if not, then logs why to stdout.
fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}
