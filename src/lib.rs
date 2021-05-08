#![feature(async_closure)] // audio.rs
#![feature(in_band_lifetimes)] // audio.rs
#![feature(map_first_last)] // audio.rs
#![deny(unused_must_use)] // because i suck at `.await`ing futures
#![deny(unused_imports)] // so as not to pollute compiling output
use crate::{
    cmd_info::CMD_INFO_COMMAND,
    cmd_join::CMD_JOIN_COMMAND,
    cmd_ping::CMD_PING_COMMAND,
    cmd_prefix::CMD_PREFIX_COMMAND,
    cmd_setup::CMD_SETUP_COMMAND,
    cmd_status::CMD_STATUS_COMMAND,
    globals::{BotConfig, BotInfo},
};
use serenity::{
    builder::CreateEmbed,
    client::Context,
    framework::standard::macros::group,
    model::{channel::Message, misc::Mentionable},
};
use std::{env, fmt::Display, io::Write};

/// The module for binding the bot to a VC
pub mod bind;
/// The module for error handling of the commands
pub mod cmd_error;
/// The module for the `help` command
pub mod cmd_help;
/// The module for the `info` command
pub mod cmd_info;
/// The module for the `join` command
pub mod cmd_join;
/// The module for the `ping` command
pub mod cmd_ping;
/// The module for the `prefix` command
pub mod cmd_prefix;
/// The module for the `setup` command
pub mod cmd_setup;
/// The module for the `status` command
pub mod cmd_status;
/// The module for a subclass of the Opus decoder that supports Send + Sync
pub mod decoder;
/// The module for DeepSpeech utils.
pub mod deepspeech;
/// The module for a subclass of the DeepSpeech model that supports Send + Sync
pub mod ds_model;
/// The module for the statics and structs to save the statics to
pub mod globals;
/// The module for event handlers
pub mod handlers;
/// The module for Statcord utilities
pub mod statcord;
/// The module for a few useful utilities
pub mod utils;

/// The hidden group for all the commands to be added to
/// - ONLY add your own groups to `sub_groups`
#[group("Master")]
#[sub_groups(General)]
#[help_available(false)]
struct Master;

/// The group for the commands provided by default
/// - You can add your own commands to it or change its name
/// - These commands will only run on mention or the guild prefix, not `.`
/// - You should add your own custom commands to different groups, then they'll use `.` too
/// - Make sure your groups only have commands NOT sub groups! The only group that can have sub groups is `Master`!
#[group("General Stuff")]
#[commands(cmd_info, cmd_prefix)]
struct General;

#[group("Bot Utils")]
#[commands(cmd_ping, cmd_status)]
struct Utils;

#[group("Voice Commands")]
#[commands(cmd_join)]
struct Voice;

#[group("Config Commands")]
#[commands(cmd_setup)]
struct Config;

/// 1. Sets the colour of the `embed` to `11534368` (The baseline error colour according to Material Design guidelines) if `is_error` is `true`, if not, sets it to the colour in the config
/// 2. Sends the `embed` to the `channel_id` of `reply`
/// ## Error
/// - Uses `log()` to inform why setting the colour failed and falls back to the `Default colour` (most likely `white`)
/// - Says why it couldn't send the embed in the channel in plain text (without embeds)
/// - DMs the author of `reply` if that also fails, colouring the embed with the error colour and telling them to report to the admins
/// - Uses `log()` to inform why it failed if even that fails
pub async fn send_embed(ctx: &Context, reply: &Message, is_error: bool, mut embed: CreateEmbed) {
    let channel = reply.channel_id;
    if is_error {
        embed.colour(11534368);
    } else {
        match BotConfig::get() {
            Some(config) => {
                embed.colour(config.colour());
            }
            None => log(ctx, "Couldn't get BotConfig to get colour").await,
        };
    };

    if let Err(err) = channel.send_message(ctx, |m| m.set_embed(embed)).await {
        if let Err(err) = channel
            .say(ctx, format!("Oops, couldn't send the message 🤦‍♀️: {}", err))
            .await
        {
            if let Err(err) = reply
                .author
                .dm(ctx, |m| {
                    m.embed(|e| {
                        e.colour(11534368)
                            .description(format!(
                                "{}\nLet the admins know so they can fix it\n",
                                err
                            ))
                            .title(format!(
                                "Looks like I can't send messages in {} :(",
                                reply.channel_id.mention()
                            ))
                    })
                })
                .await
            {
                log(
                    ctx,
                    format!(
                        "Couldn't even send the message to inform the commander: {}",
                        err
                    ),
                )
                .await
            }
        }
    }
}

/// DMs the owner of the bot, as in the application info, with the message
/// # Error
/// Falls back to `print_and_write()` also including why it couldn't log
pub async fn log(ctx: &Context, msg: impl Display + AsRef<[u8]>) {
    match BotInfo::get() {
        Some(info) => match info.owner().create_dm_channel(ctx).await {
            Ok(channel) => {
                if let Err(err) = channel.say(ctx, &msg).await {
                    print_and_write(format!(
                        "Couldn't DM the owner when trying to log: {}\nMessage: {}",
                        err, msg
                    ));
                }
            }
            Err(err) => print_and_write(format!(
                "Couldn't get the DM channel with the owner when trying to log: {}\nMessage: {}",
                err, msg
            )),
        },
        None => print_and_write(format!(
            "Couldn't get BotInfo when trying to log\nMessage: {}",
            msg
        )),
    };
}

/// Prints the `msg` and the timestamp and appends it (or creates if it doesn't exist) to the log file in the config
/// - The format of the message is: `8 July Sunday 21:34:54: {message}\n\n`
/// - This is used as fallback when `log()` fails
/// # Error
/// - Prints the message and why writing it failed
pub fn print_and_write(msg: impl Display) {
    let mut print_and_write = format!(
        "{}: {}\n\n",
        chrono::Utc::now().format("%e %B %A %H:%M:%S"),
        msg
    );
    println!("{}", print_and_write);

    let log_file = match BotConfig::get() {
        Some(config) => config.log_file(),
        None => {
            print_and_write += "Writing into a file named \"discord-base logs.txt\" \
            because getting BOT_CONFIG also failed\n\n";
            "discord-base logs.txt"
        }
    };

    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)
    {
        Ok(mut file) => {
            if let Err(err) = file.write(print_and_write.as_bytes()) {
                println!("Couldn't write to the log file: {}", err)
            }
        }
        Err(err) => println!("Couldn't open or create the log file: {}", err),
    }
}

/// 1. Sets the working directory to the directory of the binary, so that the config file and all are saved to the same directory as the file, as expected
/// 2. Also prints the working directory, just for info
/// # Error
/// - Prints why it couldn't change the directory
/// - Doesn't panic since the program can still run in the other directory which will be printed
pub fn set_dir() {
    match env::current_exe() {
        Ok(path) => match path.parent() {
            Some(parent) => {
                if let Err(err) = env::set_current_dir(parent) {
                    println!("Couldn't change the current directory: {}", err);
                }
            }
            None => println!("Couldn't get the directory of the exe"),
        },
        Err(err) => println!("Couldn't get the location of the exe: {}", err),
    }
    match env::current_dir() {
        Ok(dir) => println!(
            "All the files and all will be put in or read from: {}",
            dir.display()
        ),
        Err(err) => println!("Couldn't even get the current directory: {}", err),
    }
}
