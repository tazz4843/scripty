use serenity::client::Context;
use std::sync::Arc;

pub enum ContextTypes<'a> {
    NoArc(&'a Context),
    WithArc(&'a Arc<Context>),
}
