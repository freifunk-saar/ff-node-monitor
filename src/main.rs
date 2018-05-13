#![feature(plugin, crate_visibility_modifier)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate diesel;
extern crate dotenv;

mod db;
mod routes;

// Launch the rocket
fn main() {
    // Load development environments
    let _ = dotenv::dotenv();

    rocket::ignite()
        // TODO: Use Template::custom once rocket 0.4 is released, then we can e.g.
        // call `handlebars.set_strict_mode`.
        .manage(db::init_db_pool())
        .attach(rocket_contrib::Template::fairing())
        .mount("/", routes::routes())
        .launch();
}
