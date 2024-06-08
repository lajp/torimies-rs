#[derive(Queryable, Clone, Debug)]
pub struct DbVahti {
    pub id: i32,
    pub url: String,
    pub user_id: i64,
    pub last_updated: i64,
    pub site_id: i32,
    pub delivery_method: i32,
    pub key: Option<String>,
}

use crate::schema::Vahdit;

#[derive(Insertable)]
#[diesel(table_name = Vahdit)]
pub struct NewVahti {
    pub url: String,
    pub user_id: i64,
    pub last_updated: i64,
    pub site_id: i32,
    pub delivery_method: i32,
    pub key: Option<String>,
}
