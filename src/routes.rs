use rocket_contrib::Template;
use diesel::prelude::*;
use failure::Error;

use db_conn::DbConn;
use models::*;

#[get("/")]
fn index() -> Template {
    Template::render("index", &())
}

#[derive(FromForm)]
struct ListForm {
    email: String,
}

#[get("/list?<form>")]
fn list(form: ListForm, db: DbConn) -> Result<Template, Error> {
    use schema::monitors::dsl::*;

    let nodes = monitors
        .filter(email.eq(form.email.as_str()))
        .load::<Monitor>(&*db)?;
    Ok(Template::render("list", &json!({"email": form.email})))
}

pub fn routes() -> Vec<::rocket::Route> {
    routes![index, list]
}
