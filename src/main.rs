use std::sync::Arc;

use database::Database;
use futures::stream::{self, StreamExt};
use itertools::Itertools;
use serenity::all::CreateMessage;
use serenity::http::Http;
use teloxide::adaptors::throttle::Limits;
use teloxide::prelude::*;
use teloxide::types::ChatId;
use teloxide::Bot;

mod database;
mod error;
mod models;
mod schema;

#[macro_use]
extern crate tracing;
#[macro_use]
extern crate diesel;

use crate::models::DbVahti;

const MESSAGE: &'static str = r#"Uuden tori.fi-sivuston myöstä Torimiestä on päivitetty siten, että vanhoja Tori.fi vahteja ei enää tueta. Tästä syystä joudut valitettavasti luomaan kaikki aikaisemmin määrittelemäsi vahdit uudestaan. Päivitys astuu voimaan välittömästi tämän viestin jälkeen. Luettelo kaikista vahdeistasi seuraavassa viestissä.

Due to the new tori.fi-site, Torimies has been updated so that old vahti-urls for tori.fi are no longer supported. Unfortunately this means that you will have to redefine all your old vahtis. The update will be come into effect immediately after this message. A listing of your previously defined vahtis will be provided in the message below."#;

async fn broadcast_discord(http: Arc<Http>, user_id: i64, vahtis: Vec<DbVahti>) {
    println!("{} items to {}", vahtis.len(), user_id);
    let vahtis_message = vahtis
        .into_iter()
        .map(|v| v.url)
        .collect::<Vec<_>>()
        .as_slice()
        .join("\n");

    let user = http.get_user((user_id as u64).into()).await.unwrap();

    user.dm(&http, CreateMessage::new().content(MESSAGE))
        .await
        .unwrap();
    user.dm(
        &http,
        CreateMessage::new().content(format!("```\n{vahtis_message}\n```")),
    )
    .await
    .unwrap();
}

async fn broadcast_telegram(bot: Arc<Bot>, user_id: i64, vahtis: Vec<DbVahti>) {
    println!("{} items to {}", vahtis.len(), user_id);
    let vahtis_message = vahtis
        .into_iter()
        .map(|v| v.url)
        .collect::<Vec<_>>()
        .as_slice()
        .join("\n");

    let user = ChatId(user_id);

    bot.clone()
        .throttle(Limits::default())
        .send_message(user, MESSAGE)
        .await
        .unwrap();

    bot.clone()
        .throttle(Limits::default())
        .send_message(user, format!("```\n{vahtis_message}\n```"))
        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
        .await
        .unwrap();
}

#[tokio::main]
async fn main() {
    let db = Database::new().await;
    let vahtis = db.fetch_all_vahtis().await.unwrap();

    let discord_token = std::env::var("DISCORD_TOKEN").unwrap();
    let http = Arc::new(Http::new(&discord_token));

    let telegram_token = std::env::var("TELOXIDE_TOKEN").unwrap();
    let bot = Arc::new(Bot::new(&telegram_token));

    stream::iter(
        vahtis
            .into_iter()
            .group_by(|v| (v.user_id, v.delivery_method))
            .into_iter()
            .map(|g| (g, bot.clone(), http.clone()))
            .map(|(((uid, did), vs), bot, http)| async move {
                if did == 1 {
                    broadcast_discord(http, uid, vs.collect::<Vec<_>>()).await
                } else {
                    broadcast_telegram(bot, uid, vs.collect::<Vec<_>>()).await
                }
            })
            .collect::<Vec<_>>(),
    )
    .buffer_unordered(50)
    .collect::<Vec<_>>()
    .await;
}
