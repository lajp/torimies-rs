use async_trait::async_trait;
use chrono::{Local, TimeZone};
use futures::stream::{self, StreamExt};
use teloxide::adaptors::throttle::Limits;
use teloxide::prelude::*;
use teloxide::types::{InputFile, ParseMode};

use crate::delivery::Delivery;
use crate::error::Error;
use crate::vahti::VahtiItem;

pub struct Telegram {
    pub bot: Bot,
}

pub const ID: i32 = 2;
pub const NAME: &str = "telegram";

/// This is the telegram delivery client
/// There will be a separate client for handling commands
impl Telegram {
    pub async fn init() -> Result<Self, Error> {
        let token =
            std::env::var("TELOXIDE_TOKEN").expect("Expected TELOXIDE_TOKEN in the environment");
        let bot = Bot::new(token);
        Ok(Self { bot })
    }

    pub async fn destroy(self) {}
}

impl VahtiItem {
    fn format_telegram(self) -> String {
        let sellerurl = match self.site_id {
            #[cfg(feature = "tori")]
            crate::tori::ID => String::new(),
            #[cfg(feature = "huutonet")]
            crate::huutonet::ID => {
                format!("https://www.huuto.net/kayttaja/{}", self.seller_id)
            }
            i => panic!("Unsupported site_id {}", i),
        };

        let mut msg = format!(r#"<a href="{}">{}</a>"#, self.url, self.title) + "\n";
        msg.push_str((format!(r#"<b>Hinta</b>: {}€"#, self.price) + "\n").as_str());
        msg.push_str(
            (format!(
                r#"<b>Myyjä</b>: <a href="{}">{}</a>"#,
                sellerurl, self.seller_name
            ) + "\n")
                .as_str(),
        );
        msg.push_str((format!(r#"<b>Sijainti</b>: {}"#, self.location) + "\n").as_str());
        msg.push_str(
            (format!(
                r#"<b>Ilmoitus jätetty</b>: {}"#,
                Local
                    .timestamp_opt(self.published, 0)
                    .unwrap()
                    .format("%d/%m/%Y %R")
            ) + "\n")
                .as_str(),
        );
        msg.push_str((format!(r#"<b>Ilmoitustyyppi</b>: {}"#, self.ad_type) + "\n").as_str());
        msg.push_str(&format!(
            r#"<a href="{}">Avaa Hakusivu</a>"#,
            self.vahti_url.unwrap()
        ));

        msg
    }
}

#[async_trait]
impl Delivery for Telegram {
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

        let recipient = ChatId(fst.deliver_to.unwrap() as i64);

        stream::iter(items.into_iter().map(|i| async move {
            let file = if i.img_url.is_empty() {
                InputFile::file("./media/no_image.jpg")
            } else {
                InputFile::url(url::Url::parse(&i.img_url).unwrap())
            };

            self.bot
                .clone()
                .throttle(Limits::default())
                .send_photo(recipient, file)
                .caption(i.format_telegram())
                .parse_mode(ParseMode::Html)
                .await
                // FIXME: Perhaps don't ignore an error here
                .ok()
        }))
        .buffer_unordered(*crate::FUTURES_MAX_BUFFER_SIZE)
        .collect::<Vec<_>>()
        .await;

        Ok(())
    }
}
