use rocket_contrib::Template;
use rocket::State;

use diesel::prelude::*;
use failure::Error;
use lettre::{EmailTransport, SmtpTransport};
use lettre_email::EmailBuilder;
use rmp_serde::to_vec as serialize_to_vec;
use rmp_serde::from_slice as deserialize_from_slice;
use base64;

use db_conn::DbConn;
use models::*;
use action::*;
use config::Config;
use util::Request;

#[get("/")]
fn index(config: State<Config>) -> Template {
    Template::render("index", &json!({"config": &*config}))
}

#[derive(Serialize, FromForm)]
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
fn prepare_action(
    action: Action,
    config: State<Config>,
    req: Request,
) -> Result<Template, Error>
{
    // obtain bytes for signed action payload
    let signed_action = action.clone().sign(config.action_signing_key.as_slice());
    let signed_action = serialize_to_vec(&signed_action)?;
    let signed_action = base64::encode(&signed_action);

    // Generate email text. First line is user-visible sender, 2nd line subject.
    let mut url = config.root_url.join("run_action")?;
    url.query_pairs_mut()
        .append_pair("signed_action", signed_action.as_str());
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

#[derive(Serialize, FromForm)]
struct RunActionForm {
    signed_action: String,
}

#[get("/run_action?<form>")]
fn run_action(form: RunActionForm, db: DbConn) -> Result<Template, Error> {
    unimplemented!()
}

pub fn routes() -> Vec<::rocket::Route> {
    routes![index, list, prepare_action, run_action]
}
