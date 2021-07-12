use eval::Expr;
use scripty_db::PgPoolKey;
use scripty_macros::handle_serenity_error;
use serenity::{
    builder::CreateEmbed,
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
};
use std::time::SystemTime;
use tokio::runtime::Handle;

#[command("eval")]
#[owners_only]
#[description = "Evaluate some code.\n\
`_ctx`, and `_msg` are available in the context.\n\
Code is evaluated with the eval crate: https://docs.rs/eval/0.4.3/eval/"]
async fn cmd_eval(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut embed = CreateEmbed::default();
    let input = args.rest();
    embed.field("Input", format!("```\n{}\n```", &input), false);

    let (compile_time, output) = {
        let data = ctx.data.read().await;
        let db = unsafe { data.get::<PgPoolKey>().unwrap_unchecked().clone() };
        let runtime = Handle::current();

        let expr = Expr::new(input)
            // .value("_ctx", ctx.clone()) // uncommenting results in
            // E0277: the trait bound `serenity::prelude::Context: Serialize` is not satisfied
            .value("_msg", msg.clone())
            .function("sql", move |args| {
                let query = match args.get(0) {
                    Some(v) => match v.as_str() {
                        Some(s) => s,
                        None => {
                            return Err(eval::Error::Custom(
                                "failed to convert query to string".to_string(),
                            ))
                        }
                    },
                    None => {
                        return Err(eval::Error::Custom(
                            "missing argument to DB query".to_string(),
                        ))
                    }
                };
                match runtime.block_on(sqlx::query(query).execute(&db)) {
                    Ok(v) => Ok(format!("query success: {:?}", v).parse().unwrap()),
                    Err(e) => Err(eval::Error::Custom(format!(
                        "error while running query: {:?}",
                        e
                    ))),
                }
            });
        let st = SystemTime::now();
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
        if let Some(o) = output.2 {
            format!("{}ns runtime, {}ns compile time", o, compile_time)
        } else {
            format!("{}ns compile time", compile_time)
        },
        false,
    );

    if let Err(e) = msg
        .channel_id
        .send_message(&ctx, |m| {
            m.embed(|e| {
                *e = embed;
                e
            })
        })
        .await
    {
        handle_serenity_error!(e);
    }

    Ok(())
}
