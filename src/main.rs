use serenity::{
    client::bridge::gateway::GatewayIntents,
    framework::{standard::buckets::LimitedFor, StandardFramework},
    Client,
};

use scripty::globals::{set_redis, RedisClientWrapper, RedisConnectionWrapper};
use scripty::{
    cmd_error,
    cmd_help::CMD_HELP,
    cmd_prefix::prefix_check,
    globals::{set_db, BotConfig, BotInfo, CmdInfo, SqlitePoolKey},
    print_and_write, set_dir,
    utils::{ShardManagerWrapper, DECODE_TYPE},
    Handler, CONFIG_GROUP, GENERAL_GROUP, MASTER_GROUP, UTILS_GROUP, VOICE_GROUP,
};
use songbird::driver::CryptoMode;
use songbird::{
    driver::Config as DriverConfig,
    {SerenityInit, Songbird},
};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::RwLock;

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
async fn main() {
    set_dir();

    println!("Loading config...");
    BotConfig::set("config.toml");
    let config = BotConfig::get().expect("Couldn't access BOT_CONFIG to get the token");
    println!("Loaded config!");

    BotInfo::set(config.token()).await;
    let bot_info = BotInfo::get().expect("Couldn't access BOT_INFO to get the owner and bot ID");

    CmdInfo::set();

    println!("Loading DB...");
    let db = set_db().await;
    println!("Loaded DB!");

    println!("Connecting to Redis...");
    let (client, conn) = set_redis().await;
    println!("Connected to Redis!");

    let client_init_start = std::time::SystemTime::now();
    println!("Initializing client...");
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
                .on_mention(Some(bot_info.user()))
                .owners(vec![bot_info.owner()].into_iter().collect())
                .dynamic_prefix(|ctx, msg| Box::pin(prefix_check(ctx, msg)))
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
        .group(&MASTER_GROUP);

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
        .type_map_insert::<SqlitePoolKey>(db)
        .type_map_insert::<RedisClientWrapper>(client)
        .type_map_insert::<RedisConnectionWrapper>(Arc::new(RwLock::new(conn)))
        .framework(framework)
        .register_songbird_with(songbird)
        .await
        .expect("Couldn't create the client");
    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerWrapper>(Arc::new(RwLock::new(client.shard_manager.clone())))
    }
    println!(
        "Initialized client in {}ms!",
        client_init_start
            .elapsed()
            .expect("System clock rolled back!")
            .as_millis()
    );

    println!("Starting client...");
    if let Err(e) = client.start_autosharded().await {
        print_and_write(format!("Couldn't start the client: {}", e));
    }
}
