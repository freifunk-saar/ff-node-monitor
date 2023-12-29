// @generated automatically by Diesel CLI.

diesel::table! {
    monitors (id, email) {
        id -> Varchar,
        email -> Varchar,
    }
}

diesel::table! {
    nodes (id) {
        id -> Varchar,
        name -> Varchar,
        online -> Bool,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    monitors,
    nodes,
);
