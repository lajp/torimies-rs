use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Local, TimeZone};
use futures::stream::{self, StreamExt};
use serenity::builder::{
    CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter, CreateMessage,
};
use serenity::http::Http;
use serenity::model::application::ButtonStyle;
use serenity::model::colour::Color;

use crate::delivery::Delivery;
use crate::error::Error;
use crate::vahti::VahtiItem;

pub const ID: i32 = 1;
pub const NAME: &str = "discord";

pub struct Discord {
    pub http: Arc<Http>,
}

/// This is the discord delivery client
/// There will be a separate client for handling commands
impl Discord {
    pub async fn init() -> Result<Self, Error> {
        let token =
            std::env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in the environment");

        // NOTE: We don't need a serenity::Client because we don't have to listen to events.
        let http = Arc::new(Http::new(&token));
        Ok(Self { http })
    }

    pub async fn destroy(self) {}
}

impl VahtiItem {
    fn embed(self) -> CreateEmbed {
        match self.site_id {
            #[cfg(feature = "tori")]
            crate::tori::ID => {
                let color = match self.ad_type.as_str() {
                    "Myydään" => Color::DARK_GREEN,
                    "Annetaan" => Color::BLITZ_BLUE,
                    _ => Color::FADED_PURPLE,
                };

                let e = CreateEmbed::new()
                    .color(color)
                    .description(format!("[{}]({})", self.title, self.url))
                    .field("Hinta", format!("{} €", self.price), true)
                    .field("Myyjä", self.seller_name, true)
                    .field("Sijainti", &self.location, true)
                    .field(
                        "Ilmoitus Jätetty",
                        Local
                            .timestamp_opt(self.published, 0)
                            .unwrap()
                            .format("%d/%m/%Y %R")
                            .to_string(),
                        true,
                    )
                    .field("Ilmoitustyyppi", self.ad_type.to_string(), true)
                    .footer(CreateEmbedFooter::new(
                        self.vahti_url.expect("bug: impossible"),
                    ));
                if !self.img_url.is_empty() {
                    e.image(&self.img_url)
                } else {
                    e
                }
            }
            #[cfg(feature = "huutonet")]
            crate::huutonet::ID => {
                let e = CreateEmbed::new()
                    .color(Color::BLUE)
                    .description(format!("[{}]({})", self.title, self.url))
                    .field("Hinta", format!("{} €", self.price), true)
                    .field(
                        "Myyjä",
                        format!(
                            "[{}](https://www.huuto.net/kayttaja/{})",
                            &self.seller_name, self.seller_id
                        ),
                        true,
                    )
                    .field("Sijainti", &self.location, true)
                    .field(
                        "Ilmoitus Jätetty",
                        Local
                            .timestamp_opt(self.published, 0)
                            .unwrap()
                            .format("%d/%m/%Y %R")
                            .to_string(),
                        true,
                    )
                    .field("Ilmoitustyyppi", self.ad_type.to_string(), true)
                    .footer(CreateEmbedFooter::new(
                        self.vahti_url.expect("bug: impossible"),
                    ));
                if !self.img_url.is_empty() {
                    e.image(&self.img_url)
                } else {
                    e
                }
            }
            i => panic!("Unsupported site_id {}", i),
        }
    }
}

#[async_trait]
impl Delivery for Discord {
    async fn deliver(&self, items: Vec<VahtiItem>) -> Result<(), Error> {
        let Some(fst) = items.first() else {
            return Ok(());
        };

        assert!(items.iter().all(|i| i.deliver_to == fst.deliver_to));

        info!(
            "Delivering {} items to {}",
            items.len(),
            fst.deliver_to.unwrap()
        );

        // NOTE: Let's try 5 embeds per message, discord has a limit afaik, but idk
        // if the text/character limit will become an issue before the embed limit does
        let chunks: Vec<Vec<VahtiItem>> = items.chunks(5).map(|c| c.to_vec()).collect();

        let http = self.http.clone();
        let recipient = http
            .get_user(fst.deliver_to.expect("bug: impossible").into())
            .await?;

        stream::iter(chunks.iter().cloned())
            .map(|is| (is, http.clone(), recipient.clone()))
            .map(|(items, http, rec)| async move {
                let mut message = CreateMessage::new();
                for item in items {
                    message = message.add_embed(item.clone().embed());
                }
                let buttons = vec![CreateButton::new("remove_vahti")
                    .label("Poista vahti")
                    .style(ButtonStyle::Danger)];
                let row = CreateActionRow::Buttons(buttons);
                if cfg!(feature = "discord-command") {
                    message = message.components(vec![row]);
                }
                rec.dm(&http, message)
                    .await
                    // FIXME: Perhaps don't ignore an error here
                    .ok()
            })
            .buffer_unordered(*crate::FUTURES_MAX_BUFFER_SIZE)
            .collect::<Vec<_>>()
            .await;

        Ok(())
    }
}
