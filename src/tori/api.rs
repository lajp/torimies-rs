use serde_json::Value;

const TORI_PRICES: [&str; 9] = ["0", "25", "50", "75", "100", "250", "500", "1000", "2000"];

// FIXME: Use url crate to simplify this function
pub fn vahti_to_api(vahti: &str) -> String {
    let mut url = "https://api.tori.fi/api/v1.2/public/ads?".to_owned();
    let args = &vahti[vahti.find('?').unwrap() + 1..];
    let mut price_set = false;
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
                startprice = if let Ok(n) = parts[1].parse::<usize>() {
                    TORI_PRICES.iter().nth(n).unwrap_or(&parts[1]).to_owned()
                } else {
                    ""
                };

                price_set = true;
            }
            "pe" => {
                endprice = if let Ok(n) = parts[1].parse::<usize>() {
                    TORI_PRICES.iter().nth(n).unwrap_or(&parts[1]).to_owned()
                } else {
                    ""
                };

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
                    api_args.push(("region".to_string(), (reg - 100).to_string()));
                }
            }
            "ca" => {}
            _ => api_args.push((parts[0].to_string(), parts[1].to_string())),
        }
    }
    for arg in api_args {
        if arg.0.is_empty() {
            continue;
        } else {
            url += &format!("&{}={}", arg.0, arg.1);
        }
    }
    url = url.replace("%E4", "ä");
    url = url.replace("%C4", "Ä");
    url = url.replace("%F6", "ö");
    url = url.replace("%D6", "Ö");
    if price_set && (!startprice.is_empty() || !endprice.is_empty()) {
        url += &format!("&suborder={}-{}", &startprice, &endprice);
    }
    url
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
