use crate::globals::METRICS;
use prometheus::{Encoder, TextEncoder};
use tokio::sync::oneshot::{self, Receiver};
use rocket::Shutdown;
use std::hint::unreachable_unchecked;
/*
use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::{Request, request};
use rocket::request::FromRequest;
*/

#[rocket::get("/metrics")]
async fn metrics() -> Vec<u8> {
    let m = METRICS
        .get()
        .unwrap_or_else(|| unsafe { unreachable_unchecked() });
    let encoder = TextEncoder::new();

    let mut buffer = Vec::new();
    let metric_families = m.registry.gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    buffer
}

/*
struct Token(String);

#[derive(Debug)]
enum ApiTokenError {
    Missing,
    Invalid,
}

#[rocket::async_trait]
impl FromRequest<'r> for Token {
    type Error = ApiTokenError;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let token = request.headers().get_one("token");
        match token {
            Some(token) => {
                Outcome::Success(Token(token.to_string()))
            }
            None => Outcome::Failure((Status::Unauthorized, ApiTokenError::Missing)),
        }
    }
}

#[rocket::post("/run-stt")]
async fn run_stt(token: Token) -> (Status, String) {

    (Status::InternalServerError, "Something went wrong!".to_string())
}
*/

#[rocket::get("/")]
async fn root() -> &'static str {
    "This server doesn't have any content. Go away. *waves you away*"
}

async fn _start(tx: oneshot::Sender<Shutdown>) {
    let r = rocket::build()
        .mount("/", rocket::routes![metrics])
        .mount("/", rocket::routes![root])
        .ignite()
        .await
        .expect("failed to ignite server");

    tx.send(r.shutdown())
        .expect("receiver was dropped: don't do that!");

    if let Err(e) = r.launch().await {
        tracing::warn!("error while starting metrics server: {}", e)
    };
}

pub fn start() -> Receiver<Shutdown> {
    let (tx, rx) = oneshot::channel::<Shutdown>();
    tokio::spawn(_start(tx));
    rx
}
