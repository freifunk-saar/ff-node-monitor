use rocket_contrib::Template;
use rocket::request::FromForm;

use db_conn::DbConn;

#[get("/")]
fn index() -> Template {
    Template::render("index", &())
}

#[derive(FromForm)]
struct ListForm {
    email: String,
}

#[get("/list?<form>")]
fn list(form: ListForm) -> Template {
    Template::render("list", &json!({"email": form.email}))
}

pub fn routes() -> Vec<::rocket::Route> {
    routes![index, list]
}
