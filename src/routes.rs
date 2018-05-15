use rocket_contrib::Template;

use diesel::prelude::*;
use failure::Error;

use db_conn::DbConn;
use models::*;
use action::*;
use util::url_with_query;

#[get("/")]
fn index() -> Template {
    Template::render("index", &())
}

#[derive(Serialize,FromForm)]
struct ListForm {
    email: String,
}

#[get("/list?<form>")]
fn list(form: ListForm, db: DbConn) -> Result<Template, Error> {
    use schema::monitors::dsl::*;

    let nodes = monitors
        .filter(email.eq(form.email.as_str()))
        .load::<Monitor>(&*db)?;
    Ok(Template::render("list", &json!({"form": form, "nodes": nodes})))
}

#[get("/prepare_action?<action>")]
fn prepare_action(action: Action) -> Result<Template, Error> {
    // TODO: send email
    //let url = url_with_query("list".to_owned(), &[("email", action.email.as_str())]);
    Ok(Template::render("prepare_action", &json!({"action": action})))
}

pub fn routes() -> Vec<::rocket::Route> {
    routes![index, list, prepare_action]
}
