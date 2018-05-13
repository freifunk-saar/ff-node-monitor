use rocket_contrib::Template;

#[get("/<foo>")]
fn ff(foo: u32) -> String {
    format!("Hello, world! {}", foo)
}

#[get("/")]
fn index() -> Template {
    Template::render("index", &())
}

pub fn routes() -> Vec<::rocket::Route> {
    routes![ff, index]
}
