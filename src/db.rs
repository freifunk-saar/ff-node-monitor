use anyhow::Result;
use diesel::{Connection, PgConnection};
use diesel_migrations::MigrationHarness;

use rocket::fairing::{AdHoc, Fairing};
use rocket_sync_db_pools::{database, diesel};

// DB connection guard type
#[database("postgres")]
pub struct DbConn(diesel::PgConnection);

pub fn migration() -> impl Fairing {
    AdHoc::on_ignite("Run DB migrations", move |rocket| async move {
        let migrations = diesel_migrations::FileBasedMigrations::find_migrations_directory()
            .expect("could not load migrations");
        let conn = DbConn::get_one(&rocket)
            .await
            .expect("could not connect to DB for migrations");
        conn.run(move |db| {
            db.run_pending_migrations(migrations).unwrap();
        })
        .await;
        rocket
    })
}

impl DbConn {
    pub async fn run_transaction<T>(
        &self,
        f: impl FnOnce(&mut PgConnection) -> Result<T> + Send + 'static,
    ) -> Result<T>
    where
        T: Send + 'static,
    {
        self.run(move |db| db.transaction(f)).await
    }
}
