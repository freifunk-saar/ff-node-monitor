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
#[macro_use]
extern crate diesel as diesel_macros;

// FIXME: Get rid of the remaining `macro_use` once we can
#[macro_use]
mod util;
mod action;
mod config;
mod cron;
mod models;
mod routes;
mod schema;

use rocket::launch;

use diesel_migrations::MigrationHarness;
use diesel_async::{AsyncConnection, async_connection_wrapper::AsyncConnectionWrapper, RunQueryDsl, AsyncPgConnection};
use rocket_db_pools::{diesel, Connection, Database};
use rocket_dyn_templates::Template;

// DB connection guard type
#[derive(Database)]
#[database("diesel_postgres")]
struct DbPool(diesel::PgPool);
type DbConn = Connection<DbPool>;

#[launch]
fn rocket() -> _ {
    // Launch the rocket (also initializes `log` facade)
    rocket::build()
        .attach(DbPool::init())
        .attach(rocket::fairing::AdHoc::on_ignite(
            "Run DB migrations",
            |rocket| async {
                let migrations =
                    diesel_migrations::FileBasedMigrations::find_migrations_directory().expect("could not load migrations");
                let pool = DbPool::fetch(&rocket).expect("could not connect to DB for migrations");
                let conn = pool.get().await.unwrap();
                let conn: &mut AsyncPgConnection = &mut *conn;
                let mut conn = AsyncConnectionWrapper::<AsyncPgConnection>::from(conn);
                conn.run_pending_migrations(migrations).unwrap();
                rocket
            },
        ))
        .attach(config::fairing("ff-node-monitor"))
        .attach(Template::custom(|engines| {
            engines.handlebars.set_strict_mode(true);
        }))
        .mount("/static", rocket::fs::FileServer::from("static"))
        .mount("/", routes::routes())
}
