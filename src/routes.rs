use rocket_contrib::Template;
use rocket::State;

use diesel::prelude::*;
use diesel;
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
use cron;

#[get("/")]
fn index(config: State<Config>) -> Template {
    Template::render("index", &json!({"config": &config.ui}))
}

#[derive(Serialize, FromForm)]
struct ListForm {
    email: String,
}

#[get("/list?<form>")]
fn list(form: ListForm, db: DbConn) -> Result<Template, Error> {
    use schema::monitors::dsl::*;

    // TODO: Move this, probably to model.rs.
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
    let signed_action = action.clone().sign(&config.secrets.action_signing_key);
    let signed_action = serialize_to_vec(&signed_action)?;
    let signed_action = base64::encode(&signed_action);

    // Generate email text. First line is user-visible sender, 2nd line subject.
    let run_url = url_query!(config.urls.root_url.join("run_action")?,
        signed_action = signed_action);
    let email_template = Template::render("confirm_action",
        &json!({"action": action, "config": &config.ui, "url": run_url.as_str()}));
    let email_text = req.responder_body(email_template)?;
    let email_parts : Vec<&str> = email_text.splitn(3, '\n').collect();
    let (email_from, email_subject, email_body) = (email_parts[0], email_parts[1], email_parts[2]);

    // Build email
    let email = EmailBuilder::new()
        .to(action.email.as_str())
        .from((config.ui.email_from.as_str(), email_from))
        .subject(email_subject)
        .text(email_body)
        .build()?;
    // Send email
    let mut mailer = SmtpTransport::builder_unencrypted_localhost()?.build();
    mailer.send(&email)?;

    // Render
    let list_url = url_query!(config.urls.root_url.join("list")?,
        email = action.email);
    Ok(Template::render("prepare_action", &json!({
        "action": action,
        "list_url": list_url.as_str(),
    })))
}

#[derive(Serialize, FromForm)]
struct RunActionForm {
    signed_action: String,
}

#[get("/run_action?<form>")]
fn run_action(form: RunActionForm, db: DbConn, config: State<Config>) -> Result<Template, Error> {
    use schema::monitors;
    use diesel::result::{Error, DatabaseErrorKind};

    // Determine and verify action
    let signed_action = base64::decode(form.signed_action.as_str())?;
    let signed_action: SignedAction = deserialize_from_slice(signed_action.as_slice())?;
    let action = signed_action.verify(&config.secrets.action_signing_key)?;

    // Execute action
    // TODO: Move this, probably to action.rs.
    let success = match action.op {
        Operation::Add => {
            // TODO: Check if the node ID even exists
            let m = NewMonitor { node: action.node.as_str(), email: action.email.as_str() };
            let r = diesel::insert_into(monitors::table)
                .values(&m)
                .execute(&*db);
            // Handle UniqueViolation gracefully
            match r {
                Ok(_) => true,
                Err(Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => false,
                Err(e) => bail!(e),
            }
        }
        Operation::Remove => {
            use schema::monitors::dsl::*;

            let rows = monitors
                .filter(node.eq(action.node.as_str()))
                .filter(email.eq(action.email.as_str()));
            let num_deleted = diesel::delete(rows)
                .execute(&*db)?;
            num_deleted > 0
        }
    };

    // Render
    let list_url = url_query!(config.urls.root_url.join("list")?,
        email = action.email);
    Ok(Template::render("run_action", &json!({
        "action": action,
        "list_url": list_url.as_str(),
        "success": success,
    })))
}

#[get("/cron")]
fn cron(db: DbConn, config: State<Config>) -> Result<(), Error> {
    cron::update_nodes(&*db, &*config)?;
    Ok(())
}

pub fn routes() -> Vec<::rocket::Route> {
    routes![index, list, prepare_action, run_action, cron]
}
