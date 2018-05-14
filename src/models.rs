#[derive(Queryable, Serialize)]
pub struct Monitor {
    pub node: String,
    pub email: String,
}
