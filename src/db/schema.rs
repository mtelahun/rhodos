// @generated automatically by Diesel CLI.

diesel::table! {
    account (id) {
        id -> Int8,
        email -> Varchar,
        password -> Nullable<Varchar>,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    content (id) {
        id -> Int8,
        publisher_id -> Int8,
        cw -> Nullable<Varchar>,
        body -> Nullable<Varchar>,
        published -> Nullable<Bool>,
        published_at -> Nullable<Timestamp>,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(content -> account (publisher_id));

diesel::allow_tables_to_appear_in_same_query!(
    account,
    content,
);
