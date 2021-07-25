use super::*;
use serenity::framework::standard::macros::group;

#[group("General Stuff")]
#[commands(cmd_info, cmd_prefix, cmd_donate)]
struct General;

#[group("Bot Utils")]
#[commands(cmd_ping, cmd_credits, cmd_stats)]
struct Utils;

#[group("Voice Commands")]
#[commands(cmd_join)]
struct Voice;

#[group("Config Commands")]
#[commands(cmd_setup)]
struct Config;

#[group("Bot Owner Commands")]
#[commands(cmd_rejoin_all, cmd_shutdown, cmd_add_premium, cmd_eval)]
struct BotOwner;
