use tracing::instrument;

#[tokio::main]
#[instrument]
async fn main() {
    scripty_core::entrypoint().await
}
