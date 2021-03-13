use crate::globals::SqlitePoolKey;
use crate::utils::Receiver;
use core::task::Context as Context1;
use futures_core::Stream;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use serenity::client::Context;
use serenity::futures::StreamExt;
use serenity::model::channel::{Channel, GuildChannel};
use sqlx::{query, query_as, Row};
use std::task::Poll;
use tokio::time::Duration;
use tokio_tungstenite::tungstenite::{Error, Message};

#[derive(Serialize, Deserialize)]
struct TranscriptionResult {
    text: String,
    time: f64,
}

struct OutputChannel {
    output_channel: u64,
}

async fn results_background_loop(ctx: &Context, receiver: &Receiver, guild_id: u64) {
    'main: loop {
        // this isn't supposed to be called on a main thread, so don't do it!
        {
            let mut sockets = receiver.websockets.read().await;
            'sockets: for (user_id, &mut mut socket) in sockets.iter_mut() {
                let string = match socket.pop_front() {
                    Some(mut w) => {
                        'inner: loop {
                            match w.next().await {
                                Some(i) => match i {
                                    Ok(r) => match r.into_text() {
                                        Ok(p) => {
                                            break 'inner p;
                                        }
                                        Err(e) => {
                                            println!("Error while decoding text! {:?}", e);
                                        }
                                    },
                                    Err(e) => {
                                        println!("Error while getting msg! {:?}", e);
                                    }
                                },
                                _ => {}
                            };
                            /*
                            match w.next().await {
                                Poll::Ready(val) => ,
                                    None => {
                                        continue 'sockets; // something went wrong
                                                           // ignore it and carry on
                                    }
                                },
                                Poll::Pending => {
                                    tokio::time::sleep(Duration::from_millis(50)).await;
                                    continue 'inner;
                                }
                            }
                            */
                        }
                    }
                    None => {
                        // oh there's nothing to do, time to break
                        break 'sockets;
                    }
                };
                let i: TranscriptionResult = match from_str(&*string) {
                    Ok(v) => v,
                    Err(e) => {
                        println!("Failed to deserialize WebSocket data! {:?}", e);
                        continue 'sockets;
                    }
                };
                {
                    let db = ctx
                        .data
                        .read()
                        .await
                        .get::<SqlitePoolKey>()
                        .expect("Database is None!");
                    match query("SELECT output_channel FROM guilds WHERE guild_id = ?")
                        .bind(guild_id as i64)
                        .fetch_optional(db)
                        .await
                    {
                        Ok(row) => {
                            if let Some(row) = row {
                                if let Ok(channel_id) = row.try_get::<i64, _>("output_channel") {
                                    if let Some(c) =
                                        ctx.cache.guild_channel(channel_id as u64).await
                                    {
                                        c.say(ctx, format!("{} {}", user_id, i.text));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            println!("Failed to fetch from DB! {:?}", e);
                        }
                    };
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
