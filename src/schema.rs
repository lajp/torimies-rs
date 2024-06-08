// @generated automatically by Diesel CLI.

diesel::table! {
    Vahdit (id) {
        id -> Integer,
        url -> Text,
        user_id -> BigInt,
        last_updated -> BigInt,
        site_id -> Integer,
        delivery_method -> Integer,
        key -> Nullable<Text>,
    }
}
