use crate::send_embed;
use eval::Expr;
use serenity::{
    builder::CreateEmbed,
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
};
use std::time::SystemTime;

#[command("template")]
#[aliases("tmp")]
#[bucket = "general"]
#[description = "Evaluate some code.\n\
`_ctx`, and `_msg` are available in the context.\n\
Code is evaluated with the eval crate: https://docs.rs/eval/0.4.3/eval/"]
async fn cmd_eval(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut embed = CreateEmbed::default();
    let input = args.rest();
    embed.field("Input", format!("```\n{}\n```", &input), false);

    let (compile_time, output) = {
        let st = SystemTime::now();
        let expr = Expr::new(input)
            // .value("_ctx", ctx.clone()) // uncommenting results in
            // E0277: the trait bound `serenity::prelude::Context: Serialize` is not satisfied
            .value("_msg", msg.clone());
        let compiled = expr.compile();
        let compile_time = st.elapsed().expect("system clock rolled back").as_nanos();
        let output = match compiled {
            Ok(c) => {
                let st = SystemTime::now();
                let result = match c.exec() {
                    Ok(r) => (format!("Executed successfully! ```\n{}\n```", r), true),
                    Err(e) => (format!("Failed to execute! ```\n{}\n```", e), false),
                };
                (
                    result.0,
                    result.1,
                    if result.1 {
                        Some(st.elapsed().expect("system clock rolled back").as_nanos())
                    } else {
                        None
                    },
                )
            }
            Err(e) => (format!("Failed to compile! ```\n{}\n```", e), false, None),
        };
        (compile_time, output)
    };

    embed.field("Output", output.0, false);
    embed.field(
        "Duration",
        format!(
            "{}ns",
            if let Some(o) = output.2 {
                o
            } else {
                compile_time
            }
        ),
        false,
    );

    send_embed(ctx, msg, false, embed).await;
    Ok(())
}
