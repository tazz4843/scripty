use rocket::Shutdown;
use scripty_metrics::serialize_metrics;
use tokio::sync::oneshot::{self, Receiver};
/*
use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::{Request, request};
use rocket::request::FromRequest;
*/

#[rocket::get("/metrics")]
async fn metrics() -> Vec<u8> {
    serialize_metrics()
}

/*
TODO: actually implement authorization and a speech to text API

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
    #[allow(clippy::nonstandard_macro_braces)] // originates in a macro, nothing i can do
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
    let (tx, rx) = oneshot::channel();
    tokio::spawn(_start(tx));
    rx
}
