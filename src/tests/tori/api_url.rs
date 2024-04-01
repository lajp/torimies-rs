use super::API_BASE;
use crate::tori::api::vahti_to_api;

#[test]
fn no_keyword() {
    let url = "https://beta.tori.fi/recommerce/forsale/search";
    let expected = API_BASE.to_owned() + "&sort=PUBLISHED_DESC";
    assert_eq!(expected, vahti_to_api(url));
}

#[test]
fn basic_query() {
    let url = "https://beta.tori.fi/recommerce/forsale/search?q=thinkpad";
    let expected = API_BASE.to_owned() + "q=thinkpad&sort=PUBLISHED_DESC";
    assert_eq!(expected, vahti_to_api(url));
}

#[test]
fn query_with_non_ascii() {
    let url = "https://beta.tori.fi/recommerce/forsale/search?q=th%C3%B6nkpad";
    let expected = API_BASE.to_owned() + "q=th%C3%B6nkpad&sort=PUBLISHED_DESC";
    assert_eq!(expected, vahti_to_api(url));
}

#[test]
fn query_with_category() {
    let url = "https://beta.tori.fi/recommerce/forsale/search?category=0.93";
    let expected = API_BASE.to_owned() + "category=0.93&sort=PUBLISHED_DESC";
    assert_eq!(expected, vahti_to_api(url));
}

#[test]
fn query_with_price_range() {
    let url =
        "https://beta.tori.fi/recommerce/forsale/search?category=0.93&price_from=10&price_to=100";
    let expected =
        API_BASE.to_owned() + "category=0.93&price_from=10&price_to=100&sort=PUBLISHED_DESC";
    assert_eq!(expected, vahti_to_api(url));
}
