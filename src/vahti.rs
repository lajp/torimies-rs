use std::collections::BTreeMap;
use std::sync::Arc;
use regex::Regex;

use chrono::{Local, TimeZone};
use serde_json::Value;
use serenity::client::Context;
use serenity::http::Http;
use serenity::model::interactions::message_component::ButtonStyle;
use serenity::utils::Color;

use crate::extensions::ClientContextExt;
use crate::models::Vahti;
use crate::tori::parse::*;
use crate::{Database, ItemHistory, Mutex};

pub async fn new_vahti(ctx: &Context, url: &str, userid: u64) -> Result<String, anyhow::Error> {
    let db = ctx.get_db().await?;
    if db.fetch_vahti(url, userid.try_into()?).await.is_ok() {
        info!("Not adding a pre-defined Vahti {} for user {}", url, userid);
        return Ok("Vahti on jo määritelty!".to_string());
    }
    match db.add_vahti_entry(url, userid.try_into()?).await {
        Ok(_) => Ok("Vahti lisätty!".to_string()),
        Err(_) => bail!("Virhe tapahtui vahdin lisäyksessä!"),
    }
}

pub async fn remove_vahti(ctx: &Context, url: &str, userid: u64) -> Result<String, anyhow::Error> {
    let db = ctx.get_db().await?;
    if db.fetch_vahti(url, userid.try_into()?).await.is_err() {
        info!("Not removing a nonexistant vahti!");
        return Ok(
            "Kyseistä vahtia ei ole määritelty, tarkista että kirjoitit linkin oikein".to_string(),
        );
    }
    match db.remove_vahti_entry(url, userid.try_into()?).await {
        Ok(_) => Ok("Vahti poistettu!".to_string()),
        Err(_) => bail!("Virhe tapahtui vahdin poistamisessa!".to_string()),
    }
}

fn vahti_to_api(vahti: &str) -> String {
    let mut url = "https://api.tori.fi/api/v1.2/public/ads?".to_owned();
    let args = &vahti[vahti.find('?').unwrap() + 1..];
    let mut price_set = false;
    let mut region_defined = false;
    let mut startprice = "";
    let mut endprice = "";
    let mut api_args = Vec::<(String, String)>::new();
    for arg in args.split('&') {
        let mut parts: Vec<&str> = arg.split('=').collect();
        if parts.len() == 1 {
            parts.push("");
        }
        match parts[0] {
            "ps" => {
                startprice = parts[1];
                price_set = true;
            }
            "pe" => {
                endprice = parts[1];
                price_set = true;
            }
            "cg" => {
                if parts[1] != "0" {
                    api_args.push(("category".to_string(), parts[1].to_string()));
                }
            }
            "st" => api_args.push(("ad_type".to_string(), parts[1].to_string())),
            "m" => api_args.push(("area".to_string(), parts[1].to_string())),
            "w" => {
                let reg: i32 = parts[1].parse().unwrap();
                if reg >= 100 {
                    region_defined = true;
                    api_args.push(("region".to_string(), (reg - 100).to_string()));
                }
            }
            "ca" => api_args.push(("caregion".to_string(), parts[1].to_string())),
            _ => api_args.push((parts[0].to_string(), parts[1].to_string())),
        }
    }
    for arg in api_args {
        if arg.0 == "caregion" {
            if !region_defined {
                url += &format!("&{}={}", arg.0, arg.1);
            }
        } else {
            url += &format!("&{}={}", arg.0, arg.1);
        }
    }
    url = url.replace("%E4", "ä");
    url = url.replace("%C4", "Ä");
    url = url.replace("%F6", "ö");
    url = url.replace("%D6", "Ö");
    if price_set && !startprice.is_empty() && !endprice.is_empty() {
        url += &format!("&suborder={}-{}", &startprice, &endprice);
    }
    url
}

pub async fn is_valid_url(url: &str) -> bool {
    let tori_regex = Regex::new(r"^https://(m\.|www\.)?tori\.fi/.*\?.*$").unwrap();
    if !tori_regex.is_match(url) {
        info!("Ignoring invalid url: {}", url);
        return false;
    }
    let url = vahti_to_api(url) + "&lim=0";
    let response = reqwest::get(&url).await.unwrap().text().await.unwrap();
    let response_json: Value = serde_json::from_str(&response).unwrap();
    if let Some(counter_map) = response_json["counter_map"].as_object() {
        if let Some(amount) = counter_map["all"].as_i64() {
            amount > 0
        } else {
            false
        }
    } else {
        false
    }
}

pub async fn update_all_vahtis(
    db: Database,
    itemhistory: Arc<Mutex<ItemHistory>>,
    http: Arc<Http>,
) -> Result<(), anyhow::Error> {
    itemhistory.lock().await.purge_old();
    let vahtis = db.fetch_all_vahtis_group().await?;
    update_vahtis(db, itemhistory, http, vahtis).await?;
    Ok(())
}

pub async fn update_vahtis(
    db: Database,
    itemhistory: Arc<Mutex<ItemHistory>>,
    httpt: Arc<Http>,
    vahtis: BTreeMap<String, Vec<(i64, i64)>>,
) -> Result<(), anyhow::Error> {
    for (url, ids) in vahtis {
        let http = httpt.clone();
        let db = db.clone();
        let itemhistory = itemhistory.clone();
        tokio::spawn(async move {
            let res = reqwest::get(vahti_to_api(&url))
                .await
                .unwrap()
                .text()
                .await
                .unwrap();
            for (id, last_updated) in ids {
                if let Ok(mut currentitems) = api_parse_after(&res, last_updated).await {
                    if currentitems.len() == 10 {
                        debug!("Unsure on whether we got all the items... Querying for all of them now");
                        currentitems = api_parse_after(
                            &reqwest::get(vahti_to_api(&url))
                                .await
                                .unwrap()
                                .text()
                                .await
                                .unwrap()
                                .clone(),
                            last_updated,
                        )
                        .await
                        .unwrap();
                    }
                    if currentitems.is_empty() {
                        continue;
                    }
                    for item in currentitems.iter().rev() {
                        if itemhistory.lock().await.contains(item.ad_id, id) {
                            debug!("Item {} in itemhistory! Skipping!", item.ad_id);
                            continue;
                        }
                        itemhistory.lock().await.add_item(
                            item.ad_id,
                            id,
                            chrono::Local::now().timestamp(),
                        );
                        let blacklist = db.fetch_user_blacklist(id).await.unwrap();
                        if blacklist.contains(&item.seller_id) {
                            info!(
                                "Seller {} blacklisted by user {}! Skipping!",
                                &item.seller_id, id
                            );
                            continue;
                        }
                        let user = http.get_user(id.try_into().unwrap()).await.unwrap();
                        let c = match item.ad_type.as_str() {
                            "Myydään" => Color::DARK_GREEN,
                            "Annetaan" => Color::BLITZ_BLUE,
                            _ => Color::FADED_PURPLE,
                        };
                        user.dm(&http, |m| {
                            m.embed(|e| {
                                e.color(c);
                                e.description(format!("[{}]({})", item.title, item.url));
                                e.field("Hinta", format!("{} €", item.price), true);
                                e.field(
                                    "Myyjä",
                                    format!(
                                        "[{}](https://www.tori.fi/li?&aid={})",
                                        item.seller_name, item.seller_id
                                    ),
                                    true,
                                );
                                e.field("Sijainti", item.location.clone(), true);
                                e.field(
                                    "Ilmoitus Jätetty",
                                    Local.timestamp(item.published, 0).format("%d/%m/%Y %R"),
                                    true,
                                );
                                e.field("Ilmoitustyyppi", item.ad_type.to_string(), true);
                                e.image(item.img_url.clone())
                            });
                            m.components(|c| {
                                c.create_action_row(|r| {
                                    r.create_button(|b| {
                                        b.label("Avaa ilmoitus");
                                        b.style(ButtonStyle::Link);
                                        b.url(item.url.clone())
                                    });
                                    r.create_button(|b| {
                                        b.label("Hakulinkki");
                                        b.style(ButtonStyle::Link);
                                        b.url(&url)
                                    });
                                    r.create_button(|b| {
                                        b.label("Estä myyjä");
                                        b.style(ButtonStyle::Danger);
                                        b.custom_id("block_seller")
                                    });
                                    r.create_button(|b| {
                                        b.label("Poista Vahti");
                                        b.style(ButtonStyle::Danger);
                                        b.custom_id("remove_vahti")
                                    })
                                })
                            })
                        })
                        .await
                        .unwrap();
                    }
                    db.vahti_updated(
                        Vahti {
                            user_id: id,
                            url: url.clone(),
                            id: 0,
                            last_updated,
                        },
                        Some(currentitems[0].published),
                    )
                    .await
                    .unwrap();
                }
            }
        });
    }
    Ok(())
}
