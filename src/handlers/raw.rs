use crate::metrics::{Metrics, MetricsAsync};
use serenity::{
    async_trait,
    model::prelude::Event,
    prelude::{Context, RawEventHandler},
};

pub struct RawHandler;

#[async_trait]
impl RawEventHandler for RawHandler {
    async fn raw_event(&self, ctx: Context, event: Event) {
        let metrics = ctx.data.read().await.get::<Metrics>().cloned().unwrap();

        metrics.raw_event(&ctx, &event).await;
    }
}
