use crate::{DatabaseConnection, BOT_CONFIG};
use serde::{Deserialize, Serialize};
use std::{fs, io};

#[derive(Serialize, Deserialize)]
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

impl BotConfig {
    pub fn set(config_path: &str) {
        let config: BotConfig =
            toml::from_str(&fs::read_to_string(config_path).unwrap_or_else(|err| {
                if err.kind() == io::ErrorKind::NotFound {
                    let default_cfg = BotConfig {
                        token: "AAAAAAAAAAAAAAAAAAAA.AAAAAAA.AAAAAAAAAAAAAA".to_string(),
                        log_file: "log.txt".to_string(),
                        log_guild_added: true,
                        invite: "https://scripty.imaskeleton.me/invite".to_string(),
                        github: "https://github.com/tazz4843/scripty".to_string(),
                        colour: 11771355,
                        model_path: "/home/user/deepspeech".to_string(),
                        user: "scripty".to_string(),
                        password: "scripty".to_string(),
                        db: "scripty".to_string(),
                        host: None,
                        port: None,
                        unix_socket: Some("/var/run/postgresql/".to_string()),
                    };
                    let default_cfg_str =
                        toml::to_string_pretty(&default_cfg).expect("failed to serialize config");
                    fs::write(config_path, &default_cfg_str).unwrap_or_else(|_| {
                        panic!(
                            "Couldn't write the default config, write it manually please:\n{}",
                            default_cfg_str
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
