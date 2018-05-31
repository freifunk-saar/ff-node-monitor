table! {
    monitors (id, email) {
        id -> Varchar,
        email -> Varchar,
    }
}

table! {
    nodes (id) {
        id -> Varchar,
        name -> Varchar,
        online -> Bool,
    }
}

allow_tables_to_appear_in_same_query!(
    monitors,
    nodes,
);
