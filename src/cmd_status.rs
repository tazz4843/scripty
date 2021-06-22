use serenity::{
    client::Context,
    framework::standard::{macros::command, CommandResult},
    model::prelude::Message,
};
use systemstat::Platform;

#[command("status")]
#[only_in("guilds")]
#[bucket = "expensive"]
#[description = "Displays the bot's current resource usage."]
async fn cmd_status(ctx: &Context, msg: &Message) -> CommandResult {
    let mut message = msg
        .channel_id
        .send_message(ctx, |m| m.content("Measuring resource usage..."))
        .await?;
    let (sys_load, sys_avg1, sys_avg2, sys_avg3, sys_temp, sys_up) = tokio::spawn(async move {
        let sys_info = systemstat::System::new();

        let cpu = sys_info.cpu_load_aggregate().unwrap();
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let sys_avg = sys_info.load_average().unwrap();

        (
            cpu.done().unwrap().system * 100_f32,
            sys_avg.fifteen,
            sys_avg.five,
            sys_avg.one,
            sys_info.cpu_temp().unwrap_or_else(|_| 0_f32),
            sys_info
                .uptime()
                .unwrap_or_else(|_| std::time::Duration::new(0, 0)),
        )
    })
    .await
    .unwrap();

    let proc = std::process::id().to_string();
    let proc_usage = String::from_utf8(tokio::process::Command::new("sh")
        .arg("-c").arg(format!("pmap {} | grep bot | awk 'NR>1 {{sum+=substr($2, 1, length($2)-1)}} END {{print sum}}'", proc))
        .output().await.expect("Failed to get process memory usage").stdout).unwrap();

    let total_usage = String::from_utf8(
        tokio::process::Command::new("sh")
            .arg("-c")
            .arg(format!(
                "pmap {} | tail -n 1 | awk '/[0-9]K/{{print substr($2, 1, length($2)-1)}}'",
                proc
            ))
            .output()
            .await
            .expect("Failed to get total process memory usage")
            .stdout,
    )
    .unwrap();

    let sys_avg1 = match sys_avg1 {
        avg if avg >= 1_f32 => format!("Overloaded by {:.2}%", (avg - 1_f32) * 100_f32),
        avg if avg < 1_f32 => format!(
            "Idled for {:.2}% of the time",
            ((avg * 100_f32) - 100_f32) * -1_f32
        ),
        _ => format!("{}", sys_avg1),
    };
    let sys_avg2 = match sys_avg2 {
        avg if avg >= 1_f32 => format!("Overloaded by {:.2}%", (avg - 1_f32) * 100_f32),
        avg if avg < 1_f32 => format!(
            "Idled for {:.2}% of the time",
            ((avg * 100_f32) - 100_f32) * -1_f32
        ),
        _ => format!("{}", sys_avg2),
    };
    let sys_avg3 = match sys_avg3 {
        avg if avg >= 1_f32 => format!("Overloaded by {:.2}%", (avg - 1_f32) * 100_f32),
        avg if avg < 1_f32 => format!(
            "Idled for {:.2}% of the time",
            ((avg * 100_f32) - 100_f32) * -1_f32
        ),
        _ => format!("{}", sys_avg3),
    };

    let proc_usage = match proc_usage.trim().parse::<f32>() {
        Ok(m) => format!("{:.2}", m / 1000_f32),
        Err(_) => "0".to_string(),
    };

    let total_usage = match total_usage.trim().parse::<f32>() {
        Ok(m) => format!("{:.2}", m / 1000_f32),
        Err(_) => "0".to_string(),
    };

    if let Err(e) = message
        .edit(&ctx, |m| {
            m.embed(|embed| {
                embed
                    .color(0x7a3a0c)
                    .title("Current Usage")
                    .field(
                        "System CPU Load | Temp",
                        format!("{:.2}% under load | {:.2}*C", sys_load, sys_temp),
                        false,
                    )
                    .field("15 Minute Average", sys_avg1, false)
                    .field("5 Minute Average", sys_avg2, false)
                    .field("1 Minute Average", sys_avg3, false)
                    .field("System Uptime (seconds)", sys_up.as_secs(), false)
                    .field(
                        "Bot Memory Usage",
                        format!("Binary:\n{}MB\nBuffered:\n{}MB", proc_usage, total_usage),
                        false,
                    )
            })
        })
        .await
    {
        if let Err(_) = msg
            .author
            .direct_message(ctx, |m| {
                m.content(format!(
                    "Couldn't edit that message for some reason, try running the command again. {}",
                    e
                ))
            })
            .await
        {};
    };

    Ok(())
}
