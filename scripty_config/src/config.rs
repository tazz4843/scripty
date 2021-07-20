use crate::{DatabaseConnection, BOT_CONFIG};
use serde::Deserialize;
use std::{fs, io};

#[derive(Deserialize)]
pub struct BotConfig {
    token: String,
    log_file: String,
    log_guild_added: bool,
    invite: String,
    github: String,
    colour: u32,
    model_path: String,

    // DB stuff
    user: String,
    password: String,
    db: String,

    // DB connection stuff: EITHER host/port or unix_socket
    host: Option<String>,
    port: Option<u16>,
    unix_socket: Option<String>,
}

const DEFAULT_CONFIG: &str =
    "# The token of the bot: https://discordpy.readthedocs.io/en/latest/discord.html#creating-a-bot-account
token = \"TOKEN HERE\"

# The name of the file for logging stuff if it couldn't DM you
log_file = \"logs.txt\"

# If the bot should DM you when it's added to a guild: Must be either \"true\" or \"false\"!
log_guild_added = true

# The invite link for the bot: https://discordpy.readthedocs.io/en/latest/discord.html#inviting-your-bot
invite = \"https://scripty.imaskeleton.me/invite\"

# The link of the bot's repo's GitHub's page
github = \"https://github.com/tazz4843/scripty\"

# The colour utils::send_embed() will use if is_error is false: https://www.checkyourmath.com/convert/color/rgb_decimal.php
colour = 11771355

# Full path to the DeepSpeech model and scorer.
model_path = \"/home/user/deepspeech\"\

# DB login stuff: PostgreSQL
host = \"localhost\"\
port = 5432
user = \"scripty\"\
password = \"scripty\"\
db = \"scripty\"
";

impl BotConfig {
    pub fn set(config_path: &str) {
        let config: BotConfig =
            toml::from_str(&fs::read_to_string(config_path).unwrap_or_else(|err| {
                if err.kind() == io::ErrorKind::NotFound {
                    fs::write(config_path, DEFAULT_CONFIG).unwrap_or_else(|_| {
                        panic!(
                            "Couldn't write the default config, write it manually please:\n{}",
                            DEFAULT_CONFIG
                        )
                    });
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

    pub fn get() -> Option<&'static BotConfig> {
        BOT_CONFIG.get()
    }

    pub fn token(&self) -> &String {
        &self.token
    }
    pub fn log_file(&self) -> &String {
        &self.log_file
    }
    pub fn log_guild_added(&self) -> bool {
        self.log_guild_added
    }
    pub fn invite(&self) -> &String {
        &self.invite
    }
    pub fn github(&self) -> &String {
        &self.github
    }
    pub fn colour(&self) -> u32 {
        self.colour
    }
    pub fn model_path(&self) -> &String {
        &self.model_path
    }
    /// Get the database login.
    ///
    /// Returned tuple is user, password, and database respectively.
    pub fn db_login(&self) -> (&String, &String, &String) {
        (&self.user, &self.password, &self.db)
    }
    /// Get the database connection.
    ///
    /// Returned enum specifies whether to use a Unix socket or a TCP socket.
    pub fn db_connection(&self) -> DatabaseConnection {
        if let (Some(h), Some(p)) = (&self.host, self.port) {
            DatabaseConnection::TcpSocket(h.clone(), p)
        } else {
            DatabaseConnection::UnixSocket(
                self.unix_socket
                    .as_ref()
                    .expect("neither unix socket nor tcp socket were specified")
                    .clone(),
            )
        }
    }
}
