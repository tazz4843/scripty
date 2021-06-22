use scripty::{
    cmd_error,
    cmd_help::CMD_HELP,
    cmd_prefix::prefix_check,
    globals::{set_db, BotConfig, BotInfo, CmdInfo, PgPoolKey, METRICS, ReqwestClient},
    handlers::{bot::Handler, raw::RawHandler},
    metrics::Metrics,
    metrics_server, set_dir,
    utils::{ShardManagerWrapper, DECODE_TYPE},
    BOTOWNER_GROUP, CONFIG_GROUP, GENERAL_GROUP, MASTER_GROUP, UTILS_GROUP, VOICE_GROUP,
};
use serenity::{
    client::bridge::gateway::GatewayIntents,
    framework::{standard::buckets::LimitedFor, StandardFramework},
    Client,
};
use songbird::{
    driver::{Config as DriverConfig, CryptoMode},
    SerenityInit, Songbird,
};
use std::{
    hint::unreachable_unchecked,
    sync::{atomic::AtomicBool, Arc},
    time::SystemTime,
};
use tokio::sync::RwLock;
use tracing::{error, info, instrument, subscriber::set_global_default};

/// You should add your own requirements to get the bot started here
/// 1. Sets every global
/// 2. Creates the framework and the general and expensive buckets. You can add your own buckets to it or customise them. Customising anything else isn't recommended!
/// - You must add your groups there the same way!
/// 3. Creates the client with the required intents and starts it in auto sharded mode
/// - You should add more intents as you require them. Customising anything else isn't recommended!
/// # Panics
/// If getting the BotConfig, BotInfo or creating the client failed
/// # Errors
/// If starting the client failed, probably meaning an error on Discord's side
#[tokio::main]
#[instrument]
async fn main() {
    let sub = tracing_subscriber::fmt().with_level(true).finish();
    set_global_default(sub).expect("failed to set global default logger");

    set_dir();

    info!("Loading config...");
    BotConfig::set("config.toml");
    let config = BotConfig::get().expect("Couldn't access BOT_CONFIG to get the token");
    info!("Loaded config!");

    BotInfo::set(config.token()).await;
    let bot_info = BotInfo::get();

    CmdInfo::set();

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

    let http_client = reqwest::Client::builder().build().expect("failed to construct http client");

    // Here, we need to configure Songbird to decode all incoming voice packets.
    // If you want, you can do this on a per-call basis---here, we need it to
    // read the audio data that other people are sending us!
    let songbird = Songbird::serenity();
    songbird.set_config(
        DriverConfig::default()
            .decode_mode(DECODE_TYPE)
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
        .group(&MASTER_GROUP)
        .group(&BOTOWNER_GROUP);

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
    let server_shutdown = metrics_server::start()
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
    // the very last things called, after any final events have been processed
}
