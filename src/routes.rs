use rocket_contrib::Template;
use rocket::response;

use diesel::prelude::*;
use failure::Error;
use url::form_urlencoded;

use db_conn::DbConn;
use models::*;
use action::*;

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
    Ok(Template::render("list", &json!({"email": form.email, "nodes": nodes})))
}

fn list_url(email: &str) -> String {
    let mut to_url = "list?".to_string();
    let len = to_url.len();
    form_urlencoded::Serializer::for_suffix(&mut to_url, len)
        .append_pair("email", email);
    to_url
}

#[get("/action?<action>")]
fn action(action: Action) -> Result<response::Redirect, Error> {
    // TODO: send email
    Ok(response::Redirect::to(list_url(action.email.as_str()).as_str()))
}

pub fn routes() -> Vec<::rocket::Route> {
    routes![index, list, action]
}
