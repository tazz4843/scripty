use serenity::prelude::Context;
use std::sync::Arc;
// use crate::metrics::Metrics;
// use std::hint;

pub async fn metrics_counter(_ctx: Arc<Context>) {
    /*
    let vc_connections = {


        let data = ctx.data.read().await;
        data.get::<Metrics>().unwrap_or_else(|| unsafe {hint::unreachable_unchecked()}).voice_connections.set();
    };
    {
        let data = ctx.data.read().await;
    }
    */
}
