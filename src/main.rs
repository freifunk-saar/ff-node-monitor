//  ff-node-monitor -- Monitoring for Freifunk nodes
//  Copyright (C) 2018  Ralf Jung <post AT ralfj DOT de>
//
//  This program is free software: you can redistribute it and/or modify
//  it under the terms of the GNU Affero General Public License as published by
//  the Free Software Foundation, either version 3 of the License, or
//  (at your option) any later version.
//
//  This program is distributed in the hope that it will be useful,
//  but WITHOUT ANY WARRANTY; without even the implied warranty of
//  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//  GNU Affero General Public License for more details.
//
//  You should have received a copy of the GNU Affero General Public License
//  along with this program.  If not, see <https://www.gnu.org/licenses/>.

#![feature(proc_macro_hygiene, decl_macro, try_blocks)]

// FIXME: Diesel macros generate warnings
#![allow(proc_macro_derive_resolution_fallback)]

// FIXME: Get rid of the remaining `extern crate` once we can
#[macro_use] extern crate diesel as diesel_macros;

// FIXME: Get rid of the remaining `macro_use` once we can
#[macro_use] mod util;
mod routes;
mod action;
mod models;
mod schema;
mod config;
mod cron;

use rocket_contrib::{
    database,
    databases::diesel,
    templates::Template,
    serve::StaticFiles,
};

// DB connection guard type
#[database("postgres")]
struct DbConn(diesel::PgConnection);

fn main() {
    // Launch the rocket (also initializes `log` facade)
    rocket::ignite()
        .attach(DbConn::fairing())
        .attach(rocket::fairing::AdHoc::on_attach("Run DB migrations", |rocket| {
            let conn = DbConn::get_one(&rocket)
                .expect("could not connect to DB for migrations");
            diesel_migrations::run_pending_migrations(&*conn)
                .expect("failed to run migrations");
            Ok(rocket)
        }))
        .attach(config::fairing("ff-node-monitor"))
        .attach(Template::custom(|engines| {
            engines.handlebars.set_strict_mode(true);
        }))
        .mount("/static", StaticFiles::from("static"))
        .mount("/", routes::routes())
        .launch();
}
