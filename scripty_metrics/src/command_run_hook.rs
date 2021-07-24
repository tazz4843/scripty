use crate::METRICS;
use serenity::framework::standard::macros::hook;
use serenity::model::prelude::Message;
use serenity::prelude::Context;
use tracing::warn;

#[hook]
pub async fn before_hook(_: &Context, _: &Message, cmd_name: &str) -> bool {
    let metrics = match METRICS.get() {
        Some(m) => m,
        None => return true,
    };

    match cmd_name {
        "info" => metrics.commands.info.inc(),
        "prefix" => metrics.commands.prefix.inc(),
        "donate" => metrics.commands.donate.inc(),
        "ping" => metrics.commands.ping.inc(),
        "status" => metrics.commands.status.inc(),
        "credits" => metrics.commands.credits.inc(),
        "stats" => metrics.commands.stats.inc(),
        "join" => metrics.commands.join.inc(),
        "setup" => metrics.commands.setup.inc(),
        "rejoin_all" => metrics.commands.rejoin_all.inc(),
        "shutdown" => metrics.commands.shutdown.inc(),
        "add_premium" => metrics.commands.add_premium.inc(),
        "eval" => metrics.commands.eval.inc(),
        x => warn!("unknown command found: {}", x),
    };
    metrics.total_commands.inc();

    true
}
