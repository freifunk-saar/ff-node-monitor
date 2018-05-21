#![feature(plugin, crate_visibility_modifier, custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
extern crate r2d2;
extern crate r2d2_diesel;
#[macro_use] extern crate diesel;
extern crate diesel_migrations;
extern crate ring;
extern crate serde;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;
extern crate rmp_serde;
#[macro_use] extern crate failure;
extern crate url;
extern crate url_serde;
extern crate toml;
extern crate lettre;
extern crate lettre_email;
extern crate base64;
extern crate hex;
extern crate reqwest;
extern crate chrono;

#[macro_use] mod serde_enum_number;
#[macro_use] mod util;
mod db_conn;
mod routes;
mod action;
mod models;
mod schema;
mod config;
mod cron;

fn main() {
    // Launch the rocket
    rocket::ignite()
        .attach(config::fairing("ff-node-monitor"))
        // TODO: Use Template::custom once rocket 0.4 is released, then we can e.g.
        // call `handlebars.set_strict_mode`.
        .attach(rocket_contrib::Template::fairing())
        .mount("/", routes::routes())
        .launch();
}
