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

#![feature(plugin, crate_visibility_modifier, custom_derive)]
#![plugin(rocket_codegen)]

// FIXME: Diesel macros generate warnings
#![allow(proc_macro_derive_resolution_fallback)]

// FIXME: Get rid of the remaining macro_use once we can
#[macro_use] extern crate diesel;
#[macro_use] extern crate serde_derive;

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
