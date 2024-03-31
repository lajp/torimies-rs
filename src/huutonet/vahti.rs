use std::sync::LazyLock;

use async_trait::async_trait;
use regex::Regex;

pub static HUUTONET_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^https://(www\.)?huuto\.net/haku?.*$").unwrap());

use super::api::{is_valid_url, vahti_to_api};
use super::parse::api_parse_after;
use crate::error::Error;
use crate::models::DbVahti;
use crate::vahti::{Vahti, VahtiItem};
use crate::Database;

#[derive(Debug, Clone)]
pub struct HuutonetVahti {
    pub id: i32,
    pub url: String,
    pub user_id: u64,
    pub last_updated: i64,
    pub site_id: i32,
    pub delivery_method: i32,
}

#[async_trait]
impl Vahti for HuutonetVahti {
    async fn update(&mut self, db: &Database) -> Result<Vec<VahtiItem>, Error> {
        debug!("Updating {}", self.url);
        let res = reqwest::get(vahti_to_api(&self.url))
            .await?
            .text()
            .await?
            .to_string();

        let ret = api_parse_after(&res, self.last_updated)?
            .into_iter()
            .map(|mut i| {
                i.vahti_url = Some(self.url.clone());
                i.deliver_to = Some(self.user_id);
                i.delivery_method = Some(self.delivery_method);
                i
            })
            .collect::<Vec<_>>();

        if ret.is_empty() {
            return Ok(vec![]);
        }

        db.vahti_updated(self.to_db(), ret.iter().map(|i| i.published).max())
            .await?;

        Ok(ret)
    }

    fn is_valid_url(&self, url: &str) -> bool {
        HUUTONET_REGEX.is_match(url)
    }

    async fn validate_url(&self) -> Result<bool, Error> {
        Ok(is_valid_url(&self.url).await)
    }

    fn from_db(v: DbVahti) -> Result<Self, Error> {
        assert_eq!(v.site_id, super::ID);

        Ok(Self {
            id: v.id,
            url: v.url,
            user_id: v.user_id as u64,
            last_updated: v.last_updated,
            site_id: super::ID,
            delivery_method: v.delivery_method,
        })
    }

    fn to_db(&self) -> DbVahti {
        DbVahti {
            id: self.id,
            url: self.url.clone(),
            user_id: self.user_id as i64,
            last_updated: self.last_updated,
            site_id: self.site_id,
            delivery_method: self.delivery_method,
            key: None,
        }
    }
}
