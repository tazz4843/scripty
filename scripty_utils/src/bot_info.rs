use serenity::http::Http;
use serenity::model::prelude::UserId;
use std::lazy::SyncOnceCell as OnceCell;

/// The struct to hold the information found from the application so that we can set it to a static to avoid API requests
pub struct BotInfo {
    owner: UserId,
    user: UserId,
    name: String,
    description: String,
}

/// The static to hold `BotInfo`, so that it's global
static BOT_INFO: OnceCell<BotInfo> = OnceCell::new();

impl BotInfo {
    pub async fn set(token: &str) {
        let http = Http::new_with_token(token);
        let app_info = http
            .get_current_application_info()
            .await
            .expect("Couldn't set application info!");
        let name = http
            .get_current_user()
            .await
            .expect("Couldn't get current user")
            .name;

        let info = BotInfo {
            owner: UserId(661660243033456652),
            user: app_info.id,
            name,
            description: app_info.description,
        };

        BOT_INFO
            .set(info)
            .unwrap_or_else(|_| panic!("Couldn't set BotInfo to BOT_INFO"))
    }

    pub fn get() -> Option<&'static BotInfo> {
        BOT_INFO.get()
    }

    pub fn owner(&self) -> UserId {
        self.owner
    }
    pub fn user(&self) -> UserId {
        self.user
    }
    pub fn name(&self) -> &String {
        &self.name
    }
    pub fn description(&self) -> &String {
        &self.description
    }
}
