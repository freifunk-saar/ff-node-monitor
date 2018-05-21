use schema::*;

#[derive(Queryable, Serialize)]
pub struct MonitorQuery {
    pub node: String,
    pub email: String,
}

#[derive(Insertable, Identifiable)]
#[primary_key(node, email)]
pub struct Monitor<'a> {
    pub node: &'a str,
    pub email: &'a str,
}

#[derive(Queryable, Serialize)]
pub struct NodeQuery {
    pub node: String,
    pub name: String,
    pub online: bool,
}

#[derive(Insertable, Identifiable)]
#[primary_key(node)]
pub struct Node<'a> {
    pub node: &'a str,
    pub name: &'a str,
    pub online: bool,
}
