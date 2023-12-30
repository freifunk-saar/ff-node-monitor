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

mod action;
mod config;
mod cron;
mod models;
mod routes;
mod schema;
mod util;

use rocket::launch;

use diesel_migrations::MigrationHarness;
use rocket_dyn_templates::Template;
use rocket_sync_db_pools::{database, diesel};

// DB connection guard type
#[database("postgres")]
struct DbConn(diesel::PgConnection);

#[launch]
fn rocket() -> _ {
    // Launch the rocket (also initializes `log` facade)
    rocket::build()
        .attach(DbConn::fairing())
        .attach(rocket::fairing::AdHoc::on_ignite(
            "Run DB migrations",
            |rocket| async {
                let migrations =
                    diesel_migrations::FileBasedMigrations::find_migrations_directory()
                        .expect("could not load migrations");
                let conn = DbConn::get_one(&rocket)
                    .await
                    .expect("could not connect to DB for migrations");
                conn.run(move |db| {
                    db.run_pending_migrations(migrations).unwrap();
                })
                .await;
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
