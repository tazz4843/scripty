use std::collections::HashSet;

use serenity::{
    client::Context,
    framework::standard::{
        help_commands, macros::help, Args, CommandGroup, CommandResult, HelpOptions,
    },
    model::{channel::Message, id::UserId},
};

#[help("help", "commands", "cmds")]
#[suggestion_text = "**Maybe you meant one of these:**\n{}"]
#[max_levenshtein_distance(3)]
#[no_help_available_text = "I don't know this command :("]
#[usage_label = "You use it like"]
#[usage_sample_label = "For example"]
#[checks_label = "Only if"]
#[aliases_label = "You can also use"]
#[group_prefix = "Its prefix is "]
#[grouped_label = "It's in"]
#[description_label = "  "]
#[indention_prefix = "  "]
#[available_text = "You can use it in"]
#[dm_only_text = "My DMs only"]
#[guild_only_text = "Guilds only"]
#[dm_and_guild_text = "Both guilds and DMs"]
#[individual_command_tip = "Want me to explain a command? Type `help [command name]`\nYou can use `.` as the prefix if the command isn't in `General Stuff`"]
#[strikethrough_commands_tip_in_dm = ""]
#[strikethrough_commands_tip_in_guild = ""]
#[lacking_role = "Nothing"]
#[lacking_permissions = "Nothing"]
#[lacking_ownership = "Nothing"]
#[lacking_conditions = "Nothing"]
#[wrong_channel = "Nothing"]
#[embed_error_colour = "#b00020"]
#[embed_success_colour = "#b29ddb"]
async fn cmd_help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}
