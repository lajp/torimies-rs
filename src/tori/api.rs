use base64::prelude::*;
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha512;
use url::Url;

pub fn api_key(url: &str) -> String {
    let url = Url::parse(&vahti_to_api(url)).unwrap();
    let path = url.path();
    let query = "?".to_owned() + url.query().unwrap_or_default();

    let to_encode = format!("GET;{path}{query};Search-Quest;");
    let mut mac =
        Hmac::<Sha512>::new_from_slice("3b535f36-79be-424b-a6fd-116c6e69f137".as_bytes()).unwrap();
    mac.update(to_encode.as_bytes());
    let result = mac.finalize();
    BASE64_STANDARD.encode(result.into_bytes())
}

// TODO: Error handling
pub fn vahti_to_api(vahti: &str) -> String {
    let url = Url::parse(vahti).unwrap();
    format!("https://apps-gw-poc.svc.beta.tori.fi/search/SEARCH_ID_BAP_ALL?client=ANDROID&{}&sort=PUBLISHED_DESC", url.query().unwrap_or_default())
}

pub async fn is_valid_url(url: &str) -> bool {
    let url = vahti_to_api(url) + "&lim=0";
    let response = reqwest::get(&url)
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    if let Some(counter_map) = response["counter_map"].as_object() {
        if let Some(amount) = counter_map["all"].as_i64() {
            amount > 0
        } else {
            false
        }
    } else {
        false
    }
}
