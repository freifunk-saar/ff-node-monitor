table! {
    monitors (node, email) {
        node -> Varchar,
        email -> Varchar,
    }
}

table! {
    nodes (node) {
        node -> Varchar,
        name -> Varchar,
        online -> Bool,
    }
}

allow_tables_to_appear_in_same_query!(
    monitors,
    nodes,
);
