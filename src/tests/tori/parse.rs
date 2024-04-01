use std::fs::File;
use std::io::Read;

use crate::tori::parse::api_parse_after;
#[test]
fn basic_parse() {
    let mut file = File::open("testdata/tori/parse.json").expect("Test data not found");
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    assert_eq!(api_parse_after(&contents, 1712004500).unwrap().len(), 17);
}
