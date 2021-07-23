#![feature(once_cell)]
#![feature(option_result_unwrap_unchecked)]

mod handlers;

use crate::handlers::bot::Handler;
use crate::handlers::raw::RawHandler;
use scripty_commands::groups::*;
use scripty_commands::{cmd_error, prefix_check, CMD_HELP};
use scripty_config::BotConfig;
use scripty_db::{set_db, PgPoolKey};
use scripty_metrics::{Metrics, METRICS};
use scripty_utils::{set_dir, BotInfo, ReqwestClient, ShardManagerWrapper};
use serenity::{
    client::{bridge::gateway::GatewayIntents, parse_token},
    framework::{standard::buckets::LimitedFor, StandardFramework},
    Client,
};
use songbird::{
    driver::{Config as DriverConfig, CryptoMode, DecodeMode},
    SerenityInit, Songbird,
};
use std::{
    hint::unreachable_unchecked,
    sync::{atomic::AtomicBool, Arc},
    time::SystemTime,
};
use tokio::sync::RwLock;
use tracing::{error, info, subscriber::set_global_default};
use tracing_log::LogTracer;

pub async fn entrypoint() {
    LogTracer::init().expect("failed to hook into `log` crate");

    let sub = tracing_subscriber::fmt().with_level(true).finish();
    set_global_default(sub).expect("failed to set global default logger");

    set_dir();

    info!("Loading config...");
    BotConfig::set("config.toml");
    let config = BotConfig::get().expect("Couldn't access BOT_CONFIG to get the token");
    info!("Loaded config!");

    BotInfo::set(config.token()).await;
    let bot_info = BotInfo::get();

    let db = {
        info!("Loading DB...");
        let st = SystemTime::now();
        let db = set_db().await;
        info!(
            "Loaded DB in {}ms!",
            st.elapsed().expect("system clock rolled back").as_millis()
        );
        db
    };

    let metrics = {
        info!("Initializing metrics client...");
        let st = SystemTime::now();
        let metrics = Metrics::new();
        metrics.load_metrics().await;
        let metrics = Arc::new(metrics);
        METRICS
            .set(metrics.clone())
            .unwrap_or_else(|_| unsafe { unreachable_unchecked() });
        info!(
            "Initialized metrics client in {}ms!",
            st.elapsed().expect("system clock rolled back").as_millis()
        );
        metrics
    };

    let client_init_start = SystemTime::now();
    info!("Initializing client...");

    let http_client = reqwest::Client::builder()
        .build()
        .expect("failed to construct http client");

    // Here, we need to configure Songbird to decode all incoming voice packets.
    // If you want, you can do this on a per-call basis---here, we need it to
    // read the audio data that other people are sending us!
    let songbird = Songbird::serenity();
    songbird.set_config(
        DriverConfig::default()
            .decode_mode(DecodeMode::Decode)
            .crypto_mode(CryptoMode::Normal),
    );

    let framework = StandardFramework::new()
        .configure(|c| {
            c.prefix("~")
                .no_dm_prefix(true)
                .case_insensitivity(true)
                .dynamic_prefix(|ctx, msg| Box::pin(prefix_check(ctx, msg)));
            if let Some(bot_info) = bot_info {
                c.on_mention(Some(bot_info.user()))
                    .owners(vec![bot_info.owner()].into_iter().collect());
            }
            c
        })
        .on_dispatch_error(cmd_error::handle)
        .bucket("general", |b| {
            b.limit_for(LimitedFor::Channel)
                .await_ratelimits(1)
                .delay_action(cmd_error::delay_action)
                .time_span(600)
                .limit(10)
        })
        .await
        .bucket("expensive", |b| {
            b.limit_for(LimitedFor::Guild)
                .await_ratelimits(1)
                .delay_action(cmd_error::delay_action)
                .time_span(3600)
                .limit(10)
        })
        .await
        .help(&CMD_HELP)
        .group(&GENERAL_GROUP)
        .group(&UTILS_GROUP)
        .group(&VOICE_GROUP)
        .group(&CONFIG_GROUP)
        .group(&BOTOWNER_GROUP);

    let token_info = parse_token(&config.token()).expect("invalid token");
    let app_id = token_info.bot_user_id.as_u64();

    let mut client = Client::builder(&config.token())
        .intents(
            GatewayIntents::GUILD_MESSAGES
                | GatewayIntents::DIRECT_MESSAGES
                | GatewayIntents::GUILDS
                | GatewayIntents::GUILD_VOICE_STATES
                | GatewayIntents::GUILD_MEMBERS,
        )
        .event_handler(Handler {
            is_loop_running: AtomicBool::new(false),
            start_time: client_init_start,
        })
        .raw_event_handler(RawHandler)
        .type_map_insert::<PgPoolKey>(db)
        .type_map_insert::<Metrics>(Arc::clone(&metrics))
        .type_map_insert::<ReqwestClient>(http_client)
        .framework(framework)
        .register_songbird_with(songbird)
        .application_id(*app_id)
        .await
        .expect("Couldn't create the client");
    {
        let mut type_map = client.data.write().await;
        type_map.insert::<ShardManagerWrapper>(Arc::new(RwLock::new(client.shard_manager.clone())));
    }
    info!(
        "Initialized client in {}ms!",
        client_init_start
            .elapsed()
            .expect("System clock rolled back!")
            .as_millis()
    );

    info!("Starting metrics server...");
    let st = SystemTime::now();
    let server_shutdown = scripty_webserver::start()
        .await
        .unwrap_or_else(|_| unsafe { unreachable_unchecked() });
    info!(
        "Started metrics server in {}ms!",
        st.elapsed().expect("system clock rolled back").as_millis()
    );

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        shard_manager.lock().await.shutdown_all().await;
    });

    info!("Starting client...");
    if let Err(e) = client.start_autosharded().await {
        error!("Couldn't start the client: {}", e);
    }

    server_shutdown.notify();
    metrics.save_metrics().await;
}
