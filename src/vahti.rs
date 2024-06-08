use std::sync::{Arc, LazyLock, Mutex};

use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use itertools::Itertools;
use regex::Regex;

use crate::database::Database;
use crate::delivery::perform_delivery;
use crate::error::Error;
#[cfg(feature = "huutonet")]
use crate::huutonet::vahti::HuutonetVahti;
use crate::itemhistory::ItemHistory;
use crate::models::DbVahti;
#[cfg(feature = "tori")]
use crate::tori::vahti::ToriVahti;
use crate::{tori, Torimies};

static SITES: LazyLock<Vec<(&LazyLock<Regex>, i32)>> = LazyLock::new(|| {
    vec![
        #[cfg(feature = "tori")]
        (&crate::tori::vahti::TORI_REGEX, crate::tori::ID),
        #[cfg(feature = "huutonet")]
        (&crate::huutonet::vahti::HUUTONET_REGEX, crate::huutonet::ID),
    ]
});

// This is the Vahti trait, implementing it (and a couple of other things)
// provides support for a new site
#[async_trait]
pub trait Vahti
where
    Self: Sized + Send + Sync,
{
    async fn update(&mut self, db: &Database) -> Result<Vec<VahtiItem>, Error>;
    async fn validate_url(&self) -> Result<bool, Error>;
    fn is_valid_url(&self, url: &str) -> bool;
    fn from_db(v: DbVahti) -> Result<Self, Error>;
    fn to_db(&self) -> DbVahti;
}

#[derive(Clone, Debug, PartialEq)]
pub struct VahtiItem {
    pub deliver_to: Option<u64>,
    pub delivery_method: Option<i32>,
    pub site_id: i32,
    pub title: String,
    pub vahti_url: Option<String>,
    pub url: String,
    pub img_url: String,
    pub published: i64,
    pub price: i64,
    pub seller_name: String,
    pub seller_id: i32,
    pub location: String,
    pub ad_type: String,
    pub ad_id: i64,
}

pub async fn new_vahti(
    db: Database,
    url: &str,
    userid: u64,
    delivery_method: i32,
) -> Result<String, Error> {
    let Some(site_id) = SITES
        .iter()
        .find(|(r, _)| r.is_match(url))
        .map(|(_, sid)| *sid)
    else {
        return Err(Error::UnknownUrl(url.to_string()));
    };

    if db.fetch_vahti(url, userid as i64).await.is_ok() {
        info!("Not adding a pre-defined Vahti {} for user {}", url, userid);
        return Err(Error::VahtiExists);
    }

    let key = if site_id == tori::ID {
        Some(tori::api::api_key(url))
    } else {
        None
    };

    match db
        .add_vahti_entry(url, userid as i64, site_id, delivery_method, key)
        .await
    {
        Ok(_) => Ok(String::from("Vahti added succesfully")),
        Err(e) => Err(e),
    }
}

pub async fn remove_vahti(
    db: Database,
    url: &str,
    userid: u64,
    delivery_method: i32,
) -> Result<String, Error> {
    if db.fetch_vahti(url, userid as i64).await.is_err() {
        info!("Not removing a nonexistant vahti!");
        return Ok(
            "A Vahti is not defined with that url. Make sure the url is correct".to_string(),
        );
    }
    match db
        .remove_vahti_entry(url, userid as i64, delivery_method)
        .await
    {
        Ok(_) => Ok("Vahti removed!".to_string()),
        Err(e) => Err(e),
    }
}

impl Torimies {
    pub async fn update_all_vahtis(&mut self) -> Result<(), Error> {
        let vahtis = self.database.fetch_all_vahtis().await?;
        self.update_vahtis(vahtis).await?;
        Ok(())
    }

    pub async fn update_vahtis(&mut self, vahtis: Vec<DbVahti>) -> Result<(), Error> {
        info!("Updating {} vahtis", vahtis.len());
        let start = std::time::Instant::now();

        let ihs = self.itemhistorystorage.clone();

        // NOTE: pre-populate ItemHistoryStorage to prevent deadlocks on inserts
        // this must not be done concurrently and must be done while there are
        // no references (mutable or unmutable) into the ihs dashmap
        vahtis.iter().for_each(|v| {
            if !ihs.contains_key(&(v.user_id as u64, v.delivery_method)) {
                ihs.insert(
                    (v.user_id as u64, v.delivery_method),
                    Arc::new(Mutex::new(ItemHistory::new())),
                );
            }
        });

        let db = self.database.clone();
        let dm = self.delivery.clone();

        let items = stream::iter(vahtis.iter().cloned())
            .map(|v| (v, db.clone()))
            .map(|(v, db)| async move {
                match v.site_id {
                    #[cfg(feature = "tori")]
                    crate::tori::ID => {
                        let Ok(mut tv) = ToriVahti::from_db(v) else {
                            return vec![];
                        };

                        tv.update(&db).await.unwrap_or_default()
                    }
                    #[cfg(feature = "huutonet")]
                    crate::huutonet::ID => {
                        if let Ok(mut hv) = HuutonetVahti::from_db(v) {
                            hv.update(&db).await.unwrap_or_default()
                        } else {
                            vec![]
                        }
                    }
                    i => panic!("Unsupported site_id {}", i),
                }
            })
            .buffer_unordered(*crate::FUTURES_MAX_BUFFER_SIZE)
            .collect::<Vec<_>>()
            .await;

        info!("Receiving items took {}ms", start.elapsed().as_millis());

        let groups: Vec<Vec<VahtiItem>> = items
            .iter()
            .flatten()
            .group_by(|v| {
                (
                    v.deliver_to.expect("bug: impossible"),
                    v.delivery_method.expect("bug: impossible"),
                )
            })
            .into_iter()
            // FIXME: This could be parallellized
            .map(|(k, items)| {
                let mut ih = ihs
                    .get(&k)
                    .expect("bug: impossible")
                    .lock()
                    .unwrap()
                    .clone();

                let now = chrono::Utc::now().timestamp();
                items
                    .into_iter()
                    .filter(|i| ih.add_item(i.ad_id, i.site_id, now))
                    .cloned()
                    .collect()
            })
            .collect();

        stream::iter(
            groups
                .into_iter()
                .map(|v| (v, dm.clone()))
                .map(|(v, dm)| async move { perform_delivery(dm.clone(), v).await })
                .collect::<Vec<_>>(),
        )
        .buffer_unordered(*crate::FUTURES_MAX_BUFFER_SIZE)
        .collect::<Vec<_>>()
        .await;

        info!("Update took {}ms", start.elapsed().as_millis());
        Ok(())
    }
}
