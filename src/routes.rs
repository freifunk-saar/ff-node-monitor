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

use rocket::response::NamedFile;
use rocket::{State, request::Form, FromForm};
use rocket::{get, post, routes, uri};
use rocket_contrib::templates::Template;

use diesel::prelude::*;
use failure::{Error, bail};
use lettre::Transport;
use rmp_serde::to_vec as serialize_to_vec;
use rmp_serde::from_slice as deserialize_from_slice;
use base64;
use serde_json::json;
use serde_derive::Serialize;

use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::io;

use crate::db_conn::DbConn;
use crate::action::*;
use crate::models::*;
use crate::config::{Config, Renderer};
use crate::util::{EmailAddress, EmailBuilder};
use crate::cron;

#[get("/")]
fn index(renderer: Renderer) -> Result<Template, Error> {
    renderer.render("index", json!({}))
}

#[get("/list?<email>")]
fn list(email: EmailAddress, renderer: Renderer, db: DbConn) -> Result<Template, Error> {
    use crate::schema::*;

    db.transaction::<_, Error, _>(|| {
        let watched_nodes = monitors::table
            .filter(monitors::email.eq(email.as_str()))
            .left_join(nodes::table.on(monitors::id.eq(nodes::id)))
            .order_by(monitors::id)
            .load::<MonitorNodeQuery>(&*db)?;
        let all_nodes = {
            let watched_node_ids : HashSet<&str> = watched_nodes.iter()
                .filter_map(|node| node.node.as_ref())
                .map(|node| node.id.as_str())
                .collect();
            // Diesel does not support joining to a subquery so we have to do the filtering in Rust
            nodes::table
                .order_by(nodes::name)
                .load::<NodeQuery>(&*db)?
                .into_iter()
                .filter(|node| !watched_node_ids.contains(&node.id.as_ref()))
                .collect::<Vec<NodeQuery>>()
        };
        renderer.render("list", json!({
            "email": email,
            "watched_nodes": watched_nodes,
            "all_nodes": all_nodes,
        }))
    })
}

#[get("/list")]
fn list_formfail(renderer: Renderer) -> Result<Template, Error> {
    renderer.render("list_error", json!({}))
}

#[post("/prepare_action", data = "<action>")]
fn prepare_action(
    action: Form<Action>,
    config: State<Config>,
    renderer: Renderer,
    email_builder: EmailBuilder,
    db: DbConn,
) -> Result<Template, Error>
{
    use crate::schema::*;

    let action = action.into_inner();

    // obtain bytes for signed action payload
    let signed_action = action.clone().sign(&config.secrets.action_signing_key);
    let signed_action = serialize_to_vec(&signed_action)?;
    let signed_action = base64::encode(&signed_action);

    // compute some URLs
    let action_url = url_query!(config.urls.root.join("run_action")?,
        signed_action = signed_action);
    let list_url = url_query!(config.urls.root.join("list")?,
        email = action.email);

    // obtain user-readable node name
    let node = nodes::table
        .find(action.node.as_str())
        .first::<NodeQuery>(&*db).optional()?;
    let node_name = match node {
        Some(node) => node.name,
        None if action.op == Operation::Remove =>
            // Allow removing dead nodes
            action.node.clone(),
        None => {
            // Trying to add a non-existing node. Stop this.
            return renderer.render("prepare_action_error", json!({
                "action": action,
                "list_url": list_url.as_str(),
            }));
        }
    };

    // Generate email text
    let email_template = renderer.render("confirm_action", json!({
        "action": action,
        "node_name": node_name,
        "action_url": action_url.as_str(),
        "list_url": list_url.as_str(),
    }))?;
    // Build and send email
    let email = email_builder.email(email_template)?
        .to(action.email.as_str())
        .build()?;
    let mut mailer = email_builder.mailer()?;
    mailer.send(email.into())?;

    // Render
    let list_url = uri!(list: email = &action.email);
    renderer.render("prepare_action", json!({
        "action": action,
        "node_name": node_name,
        "list_url": config.urls.absolute(list_url),
    }))
}

#[derive(Serialize, FromForm)]
struct RunAction {
    signed_action: String,
}

#[get("/run_action?<form..>")]
fn run_action(
    form: Form<RunAction>,
    db: DbConn,
    renderer: Renderer,
    config: State<Config>
) -> Result<Template, Error> {
    // Determine and verify action
    let action : Result<Action, Error> = (|| {
        let signed_action = base64::decode(form.signed_action.as_str())?;
        let signed_action: SignedAction = deserialize_from_slice(signed_action.as_slice())?;
        Ok(signed_action.verify(&config.secrets.action_signing_key)?)
    })();
    let action = match action {
        Ok(a) => a,
        Err(_) => {
            return renderer.render("run_action_error", json!({}))
        }
    };

    // Execute action
    let success = action.run(&*db)?;

    // Render
    let list_url = url_query!(config.urls.root.join("list")?,
        email = action.email);
    renderer.render("run_action", json!({
        "action": action,
        "list_url": list_url.as_str(),
        "success": success,
    }))
}

#[get("/cron")]
fn cron(
    db: DbConn,
    config: State<Config>,
    renderer: Renderer,
    email_builder: EmailBuilder,
) -> Result<(), Error> {
    cron::update_nodes(&*db, &*config, renderer, email_builder)?;
    Ok(())
}

#[get("/static/<file..>")]
fn static_file(file: PathBuf) -> Result<Option<NamedFile>, Error> {
    // Using Option<...> turns errors into 404
    Ok(match NamedFile::open(Path::new("static/").join(file)) {
        Ok(x) => Some(x),
        Err(ref x) if x.kind() == io::ErrorKind::NotFound => None,
        Err(x) => bail!(x),
    })
}

pub fn routes() -> Vec<::rocket::Route> {
    routes![index, list, list_formfail, prepare_action, run_action, cron, static_file]
}
