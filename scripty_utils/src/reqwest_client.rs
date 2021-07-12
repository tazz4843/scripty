use serenity::prelude::TypeMapKey;

pub struct ReqwestClient;
impl TypeMapKey for ReqwestClient {
    type Value = reqwest::Client;
}
