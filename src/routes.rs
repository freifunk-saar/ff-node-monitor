use rocket_contrib::Template;

use db;

#[get("/<foo>")]
fn ff(foo: u32) -> String {
    format!("Hello, world! {}", foo)
}

#[get("/")]
fn index(_conn: db::Conn) -> Template {
    Template::render("index", &())
}

pub fn routes() -> Vec<::rocket::Route> {
    routes![ff, index]
}
