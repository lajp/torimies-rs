use super::API_BASE;

#[test]
fn correct_key() {
    let key = crate::tori::api::api_key(&(API_BASE.to_owned() + "&q=thinkpad&sort=PUBLISHED_DESC"));
    assert_eq!(
        key,
        "ciZoomD2KpXbOitLlkjrClj27mgz5gjQRLCy7TH/3csI4XvbEmMN+VPz5SopK78Lk/vxPHX1cQrAKZ6xQ+vdww=="
    );
}

#[test]
fn correct_key_2() {
    let key = crate::tori::api::api_key(
        &(API_BASE.to_owned() + "category=0.93&price_from=10&price_to=100&sort=PUBLISHED_DESC"),
    );
    assert_eq!(
        key,
        "/lajkurCdtrq7wJQoSGtCG4fG/GYT2UxyUI4RTqXPynMbxdA8nI2BITvRs5Abz+7/CUYbcjduTm45/Eamd33Tg=="
    );
}
