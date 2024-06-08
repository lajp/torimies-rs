use core::time::Duration;
use std::collections::BTreeMap;
use std::env;

use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, CustomizeConnection, Pool};
use serenity::prelude::TypeMapKey;

use crate::error::Error;
use crate::models::*;

#[derive(Clone)]
pub struct Database {
    database: Pool<ConnectionManager<SqliteConnection>>,
}

impl TypeMapKey for Database {
    type Value = Database;
}

#[derive(Debug)]
pub struct ConnectionOptions {
    pub enable_wal: bool,
    pub enable_foreign_keys: bool,
    pub busy_timeout: Option<Duration>,
}

impl CustomizeConnection<SqliteConnection, diesel::r2d2::Error> for ConnectionOptions {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
        (|| {
            if self.enable_wal {
                conn.batch_execute("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;
            }
            if self.enable_foreign_keys {
                conn.batch_execute("PRAGMA foreign_keys = ON;")?;
            }
            if let Some(d) = self.busy_timeout {
                conn.batch_execute(&format!("PRAGMA busy_timeout = {};", d.as_millis()))?;
            }
            Ok(())
        })()
        .map_err(diesel::r2d2::Error::QueryError)
    }
}

impl Database {
    pub async fn new() -> Database {
        dotenv::dotenv().ok();

        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        let manager = ConnectionManager::<SqliteConnection>::new(database_url);
        let database = Pool::builder()
            .max_size(16)
            .connection_customizer(Box::new(ConnectionOptions {
                enable_wal: true,
                enable_foreign_keys: false,
                busy_timeout: Some(Duration::from_secs(30)),
            }))
            .build(manager)
            .expect("Failed to create connection pool");

        Self { database }
    }

    pub async fn add_vahti_entry(
        &self,
        arg_url: &str,
        userid: i64,
        site_id: i32,
        delivery_method: i32,
        key: Option<String>,
    ) -> Result<usize, Error> {
        let time = chrono::Utc::now().timestamp();
        info!("Adding Vahti `{}` for the user {}", arg_url, userid);
        use crate::schema::Vahdit;
        let new_vahti = NewVahti {
            last_updated: time,
            url: arg_url.to_string(),
            user_id: userid,
            site_id,
            delivery_method,
            key,
        };
        Ok(diesel::insert_into(Vahdit::table)
            .values(&new_vahti)
            .execute(&mut self.database.get()?)?)
    }

    pub async fn remove_vahti_entry(
        &self,
        arg_url: &str,
        userid: i64,
        delivery: i32,
    ) -> Result<usize, Error> {
        info!("Removing Vahti `{}` from the user {}", arg_url, userid);
        use crate::schema::Vahdit::dsl::*;
        Ok(diesel::delete(
            Vahdit.filter(
                url.eq(arg_url)
                    .and(user_id.eq(userid))
                    .and(delivery_method.eq(delivery)),
            ),
        )
        .execute(&mut self.database.get()?)?)
    }

    pub async fn fetch_vahti_entries_by_url(&self, arg_url: &str) -> Result<Vec<DbVahti>, Error> {
        info!("Fetching Vahtis {}...", arg_url);
        use crate::schema::Vahdit::dsl::*;
        Ok(Vahdit
            .filter(url.eq(arg_url))
            .load::<DbVahti>(&mut self.database.get()?)?)
    }

    pub async fn fetch_vahti_entries_by_user_id(&self, userid: i64) -> Result<Vec<DbVahti>, Error> {
        info!("Fetching the Vahtis of user {}...", userid);
        use crate::schema::Vahdit::dsl::*;
        Ok(Vahdit
            .filter(user_id.eq(userid))
            .load::<DbVahti>(&mut self.database.get()?)?)
    }

    pub async fn fetch_vahti(&self, arg_url: &str, userid: i64) -> Result<DbVahti, Error> {
        info!("Fetching the user {}'s Vahti {}...", userid, arg_url);
        use crate::schema::Vahdit::dsl::*;
        Ok(Vahdit
            .filter(user_id.eq(userid).and(url.eq(arg_url)))
            .first::<DbVahti>(&mut self.database.get()?)?)
    }

    pub async fn fetch_all_vahtis(&self) -> Result<Vec<DbVahti>, Error> {
        info!("Fetching all Vahtis...");
        use crate::schema::Vahdit::dsl::*;
        Ok(Vahdit.load::<DbVahti>(&mut self.database.get()?)?)
    }

    pub async fn fetch_all_vahtis_group(&self) -> Result<BTreeMap<String, Vec<DbVahti>>, Error> {
        // FIXME: This could be done in sql
        info!("Fetching all vahtis grouping them by url");
        let vahdit = self.fetch_all_vahtis().await?;
        let ret: BTreeMap<String, Vec<DbVahti>> =
            vahdit.into_iter().fold(BTreeMap::new(), |mut acc, v| {
                acc.entry(v.url.clone()).or_default().push(v);
                acc
            });
        Ok(ret)
    }

    pub async fn vahti_updated(
        &self,
        vahti: DbVahti,
        timestamp: Option<i64>,
    ) -> Result<usize, Error> {
        info!(
            "Updating Vahti {} for the user {}",
            vahti.url, vahti.user_id
        );
        use crate::schema::Vahdit::dsl::*;
        let time = timestamp.unwrap_or_else(|| chrono::Utc::now().timestamp());
        info!("Newest item {}s ago", chrono::Utc::now().timestamp() - time);
        Ok(diesel::update(
            Vahdit.filter(
                url.eq(vahti.url)
                    .and(user_id.eq(vahti.user_id).and(last_updated.lt(time))),
            ),
        )
        .set(last_updated.eq(time))
        .execute(&mut self.database.get()?)?)
    }

    pub async fn fetch_user_blacklist(&self, userid: i64) -> Result<Vec<(i32, i32)>, Error> {
        debug!("Fetching the blacklist for user {}...", userid);
        use crate::schema::Blacklists::dsl::*;
        Ok(Blacklists
            .filter(user_id.eq(userid))
            .select((seller_id, site_id))
            .load::<(i32, i32)>(&mut self.database.get()?)?)
    }

    pub async fn add_seller_to_blacklist(
        &self,
        userid: i64,
        sellerid: i32,
        siteid: i32,
    ) -> Result<usize, Error> {
        info!(
            "Adding seller {} to the blacklist of user {}",
            sellerid, userid
        );
        use crate::schema::Blacklists;
        let new_entry = NewBlacklist {
            user_id: userid,
            seller_id: sellerid,
            site_id: siteid,
        };
        Ok(diesel::insert_into(Blacklists::table)
            .values(new_entry)
            .execute(&mut self.database.get()?)?)
    }

    pub async fn remove_seller_from_blacklist(
        &self,
        userid: i64,
        sellerid: i32,
        siteid: i32,
    ) -> Result<usize, Error> {
        info!(
            "Removing seller {} from the blacklist of user {}",
            sellerid, userid
        );
        use crate::schema::Blacklists::dsl::*;
        Ok(diesel::delete(
            Blacklists.filter(
                user_id
                    .eq(userid)
                    .and(seller_id.eq(sellerid))
                    .and(site_id.eq(siteid)),
            ),
        )
        .execute(&mut self.database.get()?)?)
    }
}
