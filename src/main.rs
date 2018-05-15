#![feature(plugin, crate_visibility_modifier, custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
extern crate r2d2;
extern crate r2d2_diesel;
#[macro_use] extern crate diesel;
extern crate diesel_migrations;
extern crate dotenv;
extern crate ring;
extern crate serde;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;
extern crate rmp_serde as rmps;
extern crate failure;
extern crate url;

#[macro_use] mod serde_enum_number;
mod db_conn;
mod routes;
mod action;
mod models;
mod schema;
mod util;

fn main() {
    // Load development environments and env vars
    let _ = dotenv::dotenv();
    let db_url = std::env::var("DATABASE_URL").expect("set DATABASE_URL to configure db connection");

    // Launch the rocket
    rocket::ignite()
        .manage(db_conn::init_db_pool(db_url.as_str()))
        // TODO: Use Template::custom once rocket 0.4 is released, then we can e.g.
        // call `handlebars.set_strict_mode`.
        .attach(rocket_contrib::Template::fairing())
        .mount("/", routes::routes())
        .launch();
}
