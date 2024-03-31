use super::models::ToriSearch;
use crate::error::Error;
use crate::vahti::VahtiItem;

pub fn api_parse_after(json: &str, after: i64) -> Result<Vec<VahtiItem>, Error> {
    let response: ToriSearch = serde_json::from_str(json)?;
    let items: Vec<_> = response
        .docs
        .into_iter()
        .map(VahtiItem::from)
        .take_while(|i| i.published > after)
        .collect();

    debug!("Parsed {} items", items.len());
    Ok(items)
}
