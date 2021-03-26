use std::{convert::TryFrom, fs, io};

use deepspeech::Model;
use once_cell::sync::OnceCell;
use redis::{aio::Connection, Client};
use serde::Deserialize;
use serenity::{http::client::Http, model::id::UserId, prelude::TypeMapKey};
use sqlx::{query, sqlite::SqliteConnectOptions, SqlitePool};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// The default config to be written when creating a config file
const DEFAULT_CONFIG: &'static str =
    "# The token of the bot: https://discordpy.readthedocs.io/en/latest/discord.html#creating-a-bot-account
token = \"TOKEN HERE\"

# The name of the file for logging stuff if it couldn't DM you
log_file = \"logs.txt\"

# If the bot should DM you when it's added to a guild: Must be either \"true\" or \"false\"!
log_guild_added = true

# The name of the file to use for database. Should end with: .sqlite, .sqlite3, .db or .db3
database_file = \"database.sqlite\"

# The invite link for the bot: https://discordpy.readthedocs.io/en/latest/discord.html#inviting-your-bot
invite = \"https://discord.com/api/oauth2/THE REST OF THE LINK HERE\"

# The link of the bot's repo's GitHub's page
github = \"https://github.com/USER NAME HERE/REPO NAME HERE\"

# The colour utils::send_embed() will use if is_error is false: https://www.checkyourmath.com/convert/color/rgb_decimal.php
colour = 11771355

# Full URI to your Redis instance
redis_uri = \"redis:///localhost\"

# Full path to the DeepSpeech model and scorer.
deepspeech_path = \"/home/user/deepspeech\"
";

/// The struct to implement TypeMapKey for, use this to get the SqlitePool from `ctx.data`
pub struct SqlitePoolKey;
impl TypeMapKey for SqlitePoolKey {
    type Value = SqlitePool;
}

pub struct RedisClientWrapper;
impl TypeMapKey for RedisClientWrapper {
    type Value = Client;
}

pub struct RedisConnectionWrapper;
impl TypeMapKey for RedisConnectionWrapper {
    type Value = Arc<RwLock<Connection>>;
}

/// 1. Opens a connection pool to the database file at the config file, creating it if it doesn't exist
/// 2. Runs the query given, creating the `prefixes` table (You should add your own things to it to prepare the database)
/// - DO NOT modify the `prefixes` table yourself!
/// # Panics
/// - If BotConfig isn't initialised
/// - Or if connecting to it failed
pub async fn set_db() -> SqlitePool {
    let db_filename = BotConfig::get()
        .expect("Couldn't get BOT_CONFIG to get the database file")
        .database_file
        .as_str();
    let db = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(db_filename)
            .create_if_missing(true),
    )
    .await
    .expect("Couldn't connect to the database");

    query(
        "CREATE TABLE IF NOT EXISTS prefixes (
        guild_id INTEGER PRIMARY KEY,
        prefix TEXT
    ) WITHOUT ROWID",
    )
    .execute(&db)
    .await
    .expect("Couldn't create the prefix table.");

    query(
        "CREATE TABLE IF NOT EXISTS guilds (
        guild_id INTEGER PRIMARY KEY,
        default_bind INTEGER,
        output_channel INTEGER
    ) WITHOUT ROWID",
    )
    .execute(&db)
    .await
    .expect("Couldn't create the guild table.");

    query(
        "CREATE TABLE IF NOT EXISTS users (
        user_id INTEGER PRIMARY KEY
    ) WITHOUT ROWID",
    )
    .execute(&db)
    .await
    .expect("Couldn't create the guild table.");

    db
}

/// 1. Opens a client to the Redis database.
/// 2. Opens a connection to the Redis database.
/// # Panics
/// - If BotConfig isn't initialised
/// - Or if initializing it failed
/// - Or if connecting to it failed
pub async fn set_redis() -> (Client, Connection) {
    let redis_uri = BotConfig::get()
        .expect("Couldn't get BOT_CONFIG to get the database file")
        .redis_uri
        .as_str();

    let client = Client::open(redis_uri).expect("Failed to initialize Redis client.");
    let conn = client
        .get_async_connection()
        .await
        .expect("Failed to initialize Redis connection.");
    (client, conn)
}

//noinspection SpellCheckingInspection
pub async fn set_model() -> Model {
    let model_dir_str = BotConfig::get()
        .expect("Couldn't get BOT_CONFIG to get the model path")
        .model_path
        .as_str();

    let dir_path = Path::new(&model_dir_str);
    let mut graph_name: Box<Path> = dir_path.join("output_graph.pb").into_boxed_path();
    let mut scorer_name: Option<Box<Path>> = None;
    // search for model in model directory
    for file in dir_path
        .read_dir()
        .expect("Specified model dir is not a dir")
    {
        if let Ok(f) = file {
            let file_path = f.path();
            if file_path.is_file() {
                if let Some(ext) = file_path.extension() {
                    if ext == "pb" || ext == "pbmm" {
                        graph_name = file_path.into_boxed_path();
                    } else if ext == "scorer" {
                        scorer_name = Some(file_path.into_boxed_path());
                    }
                }
            }
        }
    }
    let mut m = Model::load_from_files(&graph_name).expect("Failed to load model!");
    // enable external scorer if found in the model folder
    if let Some(scorer) = scorer_name {
        println!(
            "Using external scorer `{}`",
            scorer
                .to_str()
                .expect("Failed to convert scorer to string!")
        );
        m.enable_external_scorer(&scorer)
            .expect("Failed to initalize scorer!");
    }

    m
}

/// The struct to hold the values in the config file
#[derive(Deserialize)]
pub struct BotConfig {
    token: String,
    log_file: String,
    log_guild_added: bool,
    database_file: String,
    invite: String,
    github: String,
    colour: u32,
    redis_uri: String,
    model_path: String,
}

/// The static to hold the struct, so that it's global
static BOT_CONFIG: OnceCell<BotConfig> = OnceCell::new();

impl BotConfig {
    /// Serialises the values in the config file at the `config_path` to `BotConfig` and saves it to `BOT_CONFIG` or creates the file at `config_path` and writes `DEFAULT_CONFIG` to it if it doesn't exist
    /// - You can change the `config_path` here to customise, using directories or something other than `.toml` as the extension isn't recommended!
    /// # Panics
    /// - If the file wasn't found, also creating the file and telling to edit it
    /// - If the file couldn't be created, also printing `DEFAULT_CONFIG` and telling to write it manually
    /// - If reading the file to string failed for another reason
    /// - If parsing the file to `BotConfig` failed, meaning the file is written wrong, also telling to fix it
    /// - If saving it to BOT_CONFIG failed
    pub fn set(config_path: &str) {
        let config: BotConfig =
            toml::from_str(&fs::read_to_string(config_path).unwrap_or_else(|err| {
                if err.kind() == io::ErrorKind::NotFound {
                    fs::write(config_path, DEFAULT_CONFIG).expect(&format!(
                        "Couldn't write the default config, write it manually please:\n{}",
                        DEFAULT_CONFIG
                    ));
                    panic!("Created the default config, edit it and restart please");
                } else {
                    panic!("{}", err)
                }
            }))
            .expect("Looks like something is wrong with your config");

        BOT_CONFIG
            .set(config)
            .unwrap_or_else(|_| panic!("Couldn't set the config to BOT_CONFIG"));
    }

    /// The getter for BOT_CONFIG, returning a shared reference to the static, wrapped in `Option`
    /// # Errors
    /// Returns `None` if getting it failed, meaning it wasn't initialised
    pub fn get() -> Option<&'static BotConfig> {
        BOT_CONFIG.get()
    }

    /// The getter for the `token` field, to be used with `get()`
    pub fn token(&self) -> &String {
        &self.token
    }
    /// The getter for the `log_file` field, to be used with `get()`
    pub fn log_file(&self) -> &String {
        &self.log_file
    }
    /// The getter for the `log_guild_added` field, to be used with `get()`
    pub fn log_guild_added(&self) -> bool {
        self.log_guild_added
    }
    /// The getter for the `invite` field, to be used with `get()`
    pub fn invite(&self) -> &String {
        &self.invite
    }
    /// The getter for the `github` field, to be used with `get()`
    pub fn github(&self) -> &String {
        &self.github
    }
    /// The getter for the `colour` field, to be used with `get()`
    pub fn colour(&self) -> u32 {
        self.colour
    }
    /// The getter for the `redis_uri` field, to be used with `get()`
    pub fn redis_uri(&self) -> &String {
        &self.redis_uri
    }
    /// The getter for the `model_path` field, to be used with `get()`
    pub fn model_path(&self) -> &String {
        &self.model_path
    }
}

/// The struct to hold the information found from the application so that we can set it to a static to avoid API requests
pub struct BotInfo {
    owner: UserId,
    user: UserId,
    name: String,
    description: String,
}

/// The static to hold `BotInfo`, so that it's global
static BOT_INFO: OnceCell<BotInfo> = OnceCell::new();

impl BotInfo {
    /// 1. Creates an Http instance with the `token`
    /// 2. Gets the application info and bot user from that Http instance
    /// 3. From the current application info, gets the UserIds of the owner and the bot, and the
    /// description of the application
    /// 4. And the username of the bot from the bot user
    /// 5. And sets them to BotInfo, and saves it to BOT_INFO
    /// # Panics
    /// - If getting the application info failed
    /// - If getting the current user failed
    /// - If saving BotInfo to BOT_INFO failed
    pub async fn set(token: &str) {
        let http = Http::new_with_token(token);
        let app_info = http
            .get_current_application_info()
            .await
            .expect("Couldn't get application info");
        let name = http
            .get_current_user()
            .await
            .expect("Couldn't get current user")
            .name;

        let info = BotInfo {
            owner: UserId(661660243033456652),
            user: app_info.id,
            name,
            description: app_info.description,
        };

        BOT_INFO
            .set(info)
            .unwrap_or_else(|_| panic!("Couldn't set BotInfo to BOT_INFO"))
    }

    /// The getter for BOT_INFO, returning a shared reference to the static, wrapped in `Option`
    /// # Errors
    /// Returns `None` if getting it failed, meaning it wasn't initialised
    pub fn get() -> Option<&'static BotInfo> {
        BOT_INFO.get()
    }

    /// The getter for the `owner` field, to be used with `get()`
    pub fn owner(&self) -> UserId {
        self.owner
    }
    /// The getter for the `user` field, to be used with `get()`
    pub fn user(&self) -> UserId {
        self.user
    }
    /// The getter for the `name` field, to be used with `get()`
    pub fn name(&self) -> &String {
        &self.name
    }
    /// The getter for the `description` field, to be used with `get()`
    pub fn description(&self) -> &String {
        &self.description
    }
}

/// The struct to hold the information about commands, found from the `Master` group so that we can set it to a static to avoid iterating every time
pub struct CmdInfo {
    cmds: Vec<&'static str>,
    longest_len: u8,
    custom_cmds: Vec<&'static str>,
}

/// The static to hold `BotInfo`, so that it's global
static CMD_INFO: OnceCell<CmdInfo> = OnceCell::new();

impl CmdInfo {
    /// 1. Iterates through the sub groups of `Master`, flattening their commands' names and adding it to `cmds` and to `custom_cmds` if its group's name isn't `General Stuff`
    /// 2. Gets the command with the longest characters, saves its character count to `longest_len`
    /// 3. Creates a CmdInfo from them and saves it to `CMD_INFO`
    /// # Panics
    /// - If there are no commands
    /// - If the command's name is too long (It shouldn't be over 10 characters anyway)
    /// - If setting it to CMD_INFO failed
    pub fn set() {
        let mut cmds = vec!["help"];
        let mut custom_cmds = Vec::new();

        for group in crate::MASTER_GROUP.options.sub_groups.iter() {
            let group_cmds = group.options.commands.iter().flat_map(|c| c.options.names);
            if group.name != "General Stuff" {
                custom_cmds.extend(group_cmds.clone())
            }
            cmds.extend(group_cmds);
        }

        let longest_len = u8::try_from(
            cmds.iter()
                .map(|s| s.chars().count())
                .max()
                .expect("No commands found"),
        )
        .expect("Command name too long")
            + 10;

        CMD_INFO
            .set(CmdInfo {
                cmds,
                longest_len,
                custom_cmds,
            })
            .unwrap_or_else(|_| panic!("Couldn't set CmdInfo to CMD_INFO"))
    }

    /// The getter for BOT_INFO, returning a shared reference to the static, wrapped in `Option`
    pub fn get() -> Option<&'static CmdInfo> {
        CMD_INFO.get()
    }

    /// The getter for the `cmds` field, to be used with `get()`
    pub fn cmds(&self) -> &Vec<&'static str> {
        &self.cmds
    }
    /// The getter for the `longest_len` field, to be used with `get()`
    pub fn longest_len(&self) -> u8 {
        self.longest_len
    }
    /// The getter for the `custom_cmds` field, to be used with `get()`
    pub fn custom_cmds(&self) -> &Vec<&'static str> {
        &self.custom_cmds
    }
}
