use scripty_audio::auto_join;
use scripty_metrics::spawn_updater_task;
use scripty_utils::START_TIME;
use serenity::model::interactions::InteractionType;
use serenity::model::prelude::{Interaction, InteractionResponseType};
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::id::GuildId,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};
use tracing::info;

pub struct Handler {
    pub is_loop_running: AtomicBool,
    pub start_time: SystemTime,
}

#[async_trait]
impl EventHandler for Handler {
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        let ctx = Arc::new(ctx);

        if !self.is_loop_running.load(Ordering::Relaxed) {
            if START_TIME.get().is_some() {
                return;
            };
            self.is_loop_running.swap(true, Ordering::Relaxed);
            info!(
                "Started client in {}ms!",
                self.start_time
                    .elapsed()
                    .expect("System clock rolled back!")
                    .as_millis()
            );

            spawn_updater_task();

            let ctx1 = Arc::clone(&ctx);
            let ctx2 = Arc::clone(&ctx);
            let _ctx3 = Arc::clone(&ctx);
            let ctx4 = Arc::clone(&ctx);
            tokio::spawn(async move {
                loop {
                    scripty_utils::do_stats_update(Arc::clone(&ctx1)).await;
                    tokio::time::sleep(Duration::from_secs(30)).await;
                }
            });

            tokio::spawn(async move {
                loop {
                    auto_join(Arc::clone(&ctx2), false).await;
                    tokio::time::sleep(Duration::from_secs(300)).await;
                }
            });

            /*
            tokio::spawn(async move {
                loop {
                    metrics_counter(Arc::clone(&ctx3)).await;
                    tokio::time::sleep(Duration::from_secs(300)).await;
                }
            });
            */

            tokio::spawn(async move {
                loop {
                    scripty_utils::update_status(Arc::clone(&ctx4)).await;
                    tokio::time::sleep(Duration::from_secs(30)).await
                }
            });
        }
    }
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction.kind() {
            InteractionType::ApplicationCommand => {
                let interaction = unsafe { interaction.application_command().unwrap_unchecked() };
                let _ = interaction
                    .create_interaction_response(&ctx, |r| {
                        r.kind(InteractionResponseType::DeferredUpdateMessage)
                    })
                    .await;
            }
            InteractionType::MessageComponent => {
                let interaction = unsafe { interaction.message_component().unwrap_unchecked() };
                let _ = interaction
                    .create_interaction_response(&ctx, |r| {
                        r.kind(InteractionResponseType::DeferredUpdateMessage)
                    })
                    .await;
                match interaction.data.custom_id.as_str() {
                    "tos_agree"
                    | "result_id_picker_0"
                    | "result_id_picker_1"
                    | "result_id_picker_2"
                    | "result_id_picker_3"
                    | "result_id_picker_4"
                    | "result_id_picker_overflow"
                    | "voice_id_picker_0"
                    | "voice_id_picker_1"
                    | "voice_id_picker_2"
                    | "voice_id_picker_3"
                    | "voice_id_picker_4"
                    | "voice_id_picker_overflow" => {
                        let _ = interaction
                            .message
                            .regular()
                            .expect("not a regular message")
                            .delete(&ctx)
                            .await;
                    }
                    _ => {}
                }
            }
            _ => {}
        };
    }
}
