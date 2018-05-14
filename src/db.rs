use r2d2;
use r2d2_diesel::ConnectionManager;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use diesel_migrations;

use rocket::http::Status;
use rocket::request::{self, FromRequest};
use rocket::{Request, State, Outcome};

use std::ops;

/// Initializes a database pool.
crate type ConnMgr = ConnectionManager<PgConnection>;
crate type Pool = r2d2::Pool<ConnMgr>;

crate fn init_db_pool(db_url: &str) -> Pool {
    // Run diesel migrations
    let conn = PgConnection::establish(db_url)
        .expect("failed to connect to db");
    diesel_migrations::run_pending_migrations(&conn)
        .expect("failed to run migrations");
    // Create the pool
    let manager = ConnMgr::new(db_url);
    r2d2::Pool::builder()
        .min_idle(Some(1))
        .build(manager)
        .expect("failed to create db pool")
}

// Connection request guard type: a wrapper around an r2d2 pooled connection.
pub struct Conn(pub r2d2::PooledConnection<ConnMgr>);

/// Attempts to retrieve a single connection from the managed database pool. If
/// no pool is currently managed, fails with an `InternalServerError` status. If
/// no connections are available, fails with a `ServiceUnavailable` status.
impl<'a, 'r> FromRequest<'a, 'r> for Conn {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Conn, ()> {
        let pool = request.guard::<State<Pool>>()?;
        match pool.get() {
            Ok(conn) => Outcome::Success(Conn(conn)),
            Err(_) => Outcome::Failure((Status::ServiceUnavailable, ()))
        }
    }
}

// For the convenience of using an &DbConn as an &PgConnection.
impl ops::Deref for Conn {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
