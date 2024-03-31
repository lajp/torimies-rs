// @generated automatically by Diesel CLI.

diesel::table! {
    Blacklists (id) {
        id -> Integer,
        user_id -> BigInt,
        seller_id -> Integer,
        site_id -> Integer,
    }
}

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

diesel::allow_tables_to_appear_in_same_query!(
    Blacklists,
    Vahdit,
);
