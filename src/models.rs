use schema::*;

#[derive(Queryable, Serialize)]
pub struct MonitorQuery {
    pub id: String,
    pub email: String,
}

#[derive(Insertable, Identifiable)]
#[primary_key(id, email)]
pub struct Monitor<'a> {
    pub id: &'a str,
    pub email: &'a str,
}

#[derive(Queryable, Serialize)]
pub struct NodeQuery {
    pub id: String,
    pub name: String,
    pub online: bool,
}

#[derive(Insertable, Identifiable)]
#[primary_key(id)]
pub struct Node<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub online: bool,
}
