#[allow(unused_imports)]
use serenity::{
    builder::CreateMessage,
    model::prelude::{Mentionable, Message},
    prelude::Context,
};

pub async fn handle_message<'a, F>(ctx: &Context, msg: &Message, f: F) -> Option<Message>
where
    for<'b> F: FnOnce(&'b mut CreateMessage<'a>) -> &'b mut CreateMessage<'a>,
{
    match msg.channel_id.send_message(ctx, f).await {
        Ok(m) => Some(m),
        Err(e) => {
            if let Err(_) = msg
                .author
                .direct_message(ctx, |m| {
                    m.content(format!(
                        "I failed to send a message in {}! Make sure I have perms. {}",
                        msg.channel_id.mention(),
                        e
                    ))
                })
                .await
            {
                let _ = msg.react(ctx, '❌').await;
            };
            None
        }
    }
}

macro_rules! handle_message {
    ($ctx:expr, $msg:expr, $f:expr) => {
        match $msg.channel_id.send_message($ctx, $f).await {
            Ok(m) => Some(m),
            Err(e) => {
                if let Err(_) = $msg
                    .author
                    .direct_message($ctx, |m| {
                        m.content(format!(
                            "I failed to send a message in {}! Make sure I have perms. {}",
                            $msg.channel_id.mention(),
                            e
                        ))
                    })
                    .await
                {
                    let _ = $msg.react($ctx, '❌').await;
                };
                None
            }
        }
    };
}
