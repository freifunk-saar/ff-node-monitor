use schema::*;

#[derive(Queryable, Insertable, Serialize)]
pub struct Monitor {
    pub node: String,
    pub email: String,
}

#[derive(Insertable)]
#[table_name="monitors"]
pub struct NewMonitor<'a> {
    pub node: &'a str,
    pub email: &'a str,
}

#[derive(Queryable, Serialize)]
pub struct Node {
    pub node: String,
    pub name: String,
    pub state: bool,
}

#[derive(Insertable)]
#[table_name="nodes"]
pub struct NewNode<'a> {
    pub node: &'a str,
    pub name: &'a str,
    pub state: bool,
}
