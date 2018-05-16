use rocket_contrib::Template;
use rocket::State;

use diesel::prelude::*;
use failure::Error;
use lettre::{EmailTransport, SmtpTransport};
use lettre_email::EmailBuilder;

use db_conn::DbConn;
use models::*;
use action::*;
use config::Config;
use util::Request;

#[get("/")]
fn index(config: State<Config>) -> Template {
    Template::render("index", &json!({"config": &*config}))
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
fn prepare_action<'a, 'r>(
    action: Action,
    config: State<Config>,
    req: Request<'a, 'r>,
) -> Result<Template, Error>
{
    // Generate email text. First line is user-visible sender, 2nd line subject.
    let mut url = config.root_url.join("list")?;
    url.query_pairs_mut()
        .append_pair("email", action.email.as_str());
    let email_template = Template::render("confirm_action",
        &json!({"action": action, "config": &*config, "url": url.as_str()}));
    let email_text = req.responder_body(email_template)?;
    let email_parts : Vec<&str> = email_text.splitn(3, '\n').collect();
    let (email_from, email_subject, email_body) = (email_parts[0], email_parts[1], email_parts[2]);
    // Build email
    let email = EmailBuilder::new()
        .to(action.email.as_str())
        .from((config.email_from.as_str(), email_from))
        .subject(email_subject)
        .text(email_body)
        .build()?;
    // Send email
    let mut mailer = SmtpTransport::builder_unencrypted_localhost()?.build();
    mailer.send(&email)?;

    Ok(Template::render("prepare_action", &json!({"action": action})))
}

pub fn routes() -> Vec<::rocket::Route> {
    routes![index, list, prepare_action]
}
