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

#![feature(proc_macro_hygiene, decl_macro, crate_visibility_modifier)]

// FIXME: Diesel macros generate warnings
#![allow(proc_macro_derive_resolution_fallback)]

// FIXME: Get rid of the remaining `extern crate` once we can
#[macro_use] extern crate diesel;

// FIXME: Get rid of the remaining `macro_use` once we can
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
        .attach(rocket_contrib::templates::Template::custom(|engines| {
            engines.handlebars.set_strict_mode(true);
        }))
        .mount("/static", rocket_contrib::serve::StaticFiles::from("static"))
        .mount("/", routes::routes())
        .launch();
}
