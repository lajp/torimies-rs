#![allow(dead_code)]
use serde::Deserialize;

use crate::vahti::VahtiItem;

#[derive(Deserialize, Debug, Clone)]
pub struct ToriImage {
    pub url: String,
    pub path: String,
    pub height: i64,
    pub width: i64,
    pub aspect_ratio: f32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ToriCoordinates {
    pub lon: f32,
    pub lat: f32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ToriLabel {
    pub id: String,
    pub text: String,
    pub r#type: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ToriPrice {
    pub amount: i64,
    pub currency_code: String,
    pub price_unit: String,
}

/// An item returned by the search request
#[derive(Deserialize, Debug, Clone)]
pub struct ToriItem {
    /// Type of the item for Torimies always "bap"
    //pub r#type: String,
    /// The id same as ad_id but as a String
    pub id: String,
    /// Main search key for Torimies always "SEARCH_ID_BAP_ALL"
    pub main_search_key: String,
    /// The title of the ad
    pub heading: String,
    /// The location of the ad
    pub location: String,
    /// The main image of the ad, not present if there is no image
    #[serde(default)]
    pub image: Option<ToriImage>,
    /// No idea, seems to always contain "private"
    pub flags: Vec<String>,
    /// The timestamp of the image in milliseconds
    pub timestamp: i64,
    /// Coordinates of the image
    pub coordinates: ToriCoordinates,
    /// No idea, some ad type
    pub ad_type: u32,
    /// Labels of the ad, seems to contain
    /// { "id": "private, "text": "Yksityinen", "type": "SECONDARY" }
    pub labels: Vec<ToriLabel>,
    /// The web-url for the ad
    pub canonical_url: String,
    /// The price not present if no price
    pub price: Option<ToriPrice>,
    /// Distance to the ad?
    pub distance: f32,
    /// Trade type, e.g. "Myydään" or "Ostetaan"
    pub trade_type: String,
    /// Image urls for the ad, empty if no images
    pub image_urls: Vec<String>,
    /// Same as id but not a String
    pub ad_id: i64,
}

/// The search response JSON contains
/// the necessary items in the "docs" field
/// and then thousands of lines of categories
/// each stating how many of the items
/// belong to that category
#[derive(Deserialize, Debug, Clone)]
pub struct ToriSearch {
    /// An array of items
    pub docs: Vec<ToriItem>,
}

impl From<ToriItem> for VahtiItem {
    fn from(t: ToriItem) -> VahtiItem {
        VahtiItem {
            deliver_to: None,
            delivery_method: None,
            site_id: super::ID,
            title: t.heading,
            vahti_url: None,
            url: t.canonical_url,
            img_url: t.image.map(|i| i.url).unwrap_or_default(),
            published: t.timestamp / 1000,
            price: t.price.map(|p| p.amount).unwrap_or_default(),
            seller_name: t
                .labels
                .iter()
                .find(|l| l.r#type == "SECONDARY")
                .map(|l| l.text.clone())
                .unwrap_or(String::from("Tuntematon")),
            seller_id: 0,
            location: t.location,
            ad_type: t.trade_type,
            ad_id: t.ad_id,
        }
    }
}
