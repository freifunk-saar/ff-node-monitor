#![feature(plugin, crate_visibility_modifier)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate diesel;
extern crate diesel_migrations;
extern crate dotenv;

mod db;
mod routes;

fn main() {
    // Load development environments and env vars
    let _ = dotenv::dotenv();
    let db_url = std::env::var("DATABASE_URL").expect("set DATABASE_URL to configure db connection");

    // Launch the rocket
    rocket::ignite()
        .manage(db::init_db_pool(db_url.as_str()))
        // TODO: Use Template::custom once rocket 0.4 is released, then we can e.g.
        // call `handlebars.set_strict_mode`.
        .attach(rocket_contrib::Template::fairing())
        .mount("/", routes::routes())
        .launch();
}
