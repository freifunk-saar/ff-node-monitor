#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;

use std::collections::HashMap;

use rocket_contrib::Template;

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
        .attach(Template::fairing())
        .mount("/", routes![ff, index])
        .launch();
}
