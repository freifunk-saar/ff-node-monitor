#![feature(plugin, crate_visibility_modifier)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate diesel;

use rocket_contrib::Template;

mod db;

#[get("/<foo>")]
fn ff(foo: u32) -> String {
    format!("Hello, world! {}", foo)
}

#[get("/")]
fn index() -> Template {
    Template::render("index", &())
}

fn main() {
    rocket::ignite()
        // TODO: Use Template::custom once rocket 0.4 is released, then we can e.g.
        // call `handlebars.set_strict_mode`.
        .manage(db::init_db_pool())
        .attach(Template::fairing())
        .mount("/", routes![ff, index])
        .launch();
}
