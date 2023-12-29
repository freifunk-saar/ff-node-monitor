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

use std::collections::HashSet;

use base64::Engine as _;
use rocket::{form::Form, response::Debug, State};
use rocket::{get, post, routes, uri};
use rocket_dyn_templates::Template;

use diesel::prelude::*;
use rmp_serde::from_slice as deserialize_from_slice;
use rmp_serde::to_vec as serialize_to_vec;
use serde_json::json;

use crate::action::*;
use crate::config::{Config, Renderer};
use crate::cron;
use crate::models::*;
use crate::util::{EmailAddress, EmailSender};
use crate::DbConn;

type Result<T> = std::result::Result<T, Debug<anyhow::Error>>;

const BASE64_ENGINE: base64::engine::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;

#[get("/")]
fn index(renderer: Renderer) -> Result<Template> {
    Ok(renderer.render("index", json!({}))?)
}

#[get("/list?<email>")]
async fn list(email: EmailAddress, renderer: Renderer<'_>, db: DbConn) -> Result<Template> {
    use crate::schema::*;

    let vars = db
        .run::<_, anyhow::Result<_>>(move |db| {
            let watched_nodes = monitors::table
                .filter(monitors::email.eq(email.as_str()))
                .left_join(nodes::table.on(monitors::id.eq(nodes::id)))
                .order_by(monitors::id)
                .load::<MonitorNodeQuery>(db)?;
            let all_nodes = {
                let watched_node_ids: HashSet<&str> = watched_nodes
                    .iter()
                    .filter_map(|node| node.node.as_ref())
                    .map(|node| node.id.as_str())
                    .collect();
                // Diesel does not support joining to a subquery so we have to do the filtering in Rust
                nodes::table
                    .order_by(nodes::name)
                    .load::<NodeQuery>(db)?
                    .into_iter()
                    .filter(|node| !watched_node_ids.contains(&node.id.as_ref()))
                    .collect::<Vec<NodeQuery>>()
            };
            Ok(json!({
                "email": email,
                "watched_nodes": watched_nodes,
                "all_nodes": all_nodes,
            }))
        })
        .await?;

    Ok(renderer.render("list", vars)?)
}

#[get("/list")]
fn list_formfail(renderer: Renderer) -> Result<Template> {
    Ok(renderer.render("list_error", json!({}))?)
}

#[post("/prepare_action", data = "<action>")]
async fn prepare_action(
    action: Form<Action>,
    config: &State<Config>,
    renderer: Renderer<'_>,
    email_sender: EmailSender<'_>,
    db: DbConn,
) -> Result<Template> {
    use crate::schema::*;

    let action = action.into_inner();

    // obtain bytes for signed action payload
    let signed_action = action.clone().sign(&config.secrets.action_signing_key);
    let signed_action = serialize_to_vec(&signed_action).map_err(|e| Debug(e.into()))?;
    let signed_action = BASE64_ENGINE.encode(&signed_action);

    // compute some URLs
    let action_url = config
        .urls
        .absolute(uri!(run_action(signed_action = &signed_action)));
    let list_url = config.urls.absolute(uri!(list(email = &action.email)));

    // obtain user-readable node name
    let node = action.node.clone();
    let node = db
        .run(move |db| {
            nodes::table
                .find(node.as_str())
                .first::<NodeQuery>(db)
                .optional()
        })
        .await
        .map_err(|e| Debug(e.into()))?;
    let node_name = match node {
        Some(node) => node.name,
        None if action.op == Operation::Remove =>
        // Allow removing dead nodes
        {
            action.node.clone()
        }
        None => {
            // Trying to add a non-existing node. Stop this.
            return Ok(renderer.render(
                "prepare_action_error",
                json!({
                    "action": action,
                    "list_url": list_url.as_str(),
                }),
            )?);
        }
    };

    // Build and send email
    email_sender
        .email(
            "confirm_action",
            json!({
                "action": action,
                "node_name": node_name,
                "action_url": action_url.as_str(),
                "list_url": list_url.as_str(),
            }),
            action.email.as_str(),
        )
        .await?;

    // Render
    Ok(renderer.render(
        "prepare_action",
        json!({
            "action": action,
            "node_name": node_name,
            "list_url": list_url,
        }),
    )?)
}

#[get("/run_action?<signed_action>")]
async fn run_action(
    signed_action: String,
    db: DbConn,
    renderer: Renderer<'_>,
    config: &State<Config>,
) -> Result<Template> {
    // Determine and verify action
    let action: Result<Action> = (|| {
        let signed_action = BASE64_ENGINE
            .decode(signed_action.as_str())
            .map_err(|e| Debug(e.into()))?;
        let signed_action: SignedAction =
            deserialize_from_slice(signed_action.as_slice()).map_err(|e| Debug(e.into()))?;
        Ok(signed_action
            .verify(&config.secrets.action_signing_key)
            .map_err(|_| anyhow::anyhow!("signature verification failed"))?)
    })();
    let action = match action {
        Ok(a) => a,
        Err(_) => return Ok(renderer.render("run_action_error", json!({}))?),
    };

    // Execute action
    let success = action.run(&db).await?;

    // Render
    let list_url = config.urls.absolute(uri!(list(email = &action.email)));
    Ok(renderer.render(
        "run_action",
        json!({
            "action": action,
            "list_url": list_url,
            "success": success,
        }),
    )?)
}

#[get("/cron")]
async fn cron_route(
    db: DbConn,
    config: &State<Config>,
    renderer: Renderer<'_>,
    email_sender: EmailSender<'_>,
) -> Result<Template> {
    Ok(
        match cron::update_nodes(&db, config, email_sender).await? {
            cron::UpdateResult::NotEnoughOnline(online) => renderer.render(
                "cron_error",
                json!({
                    "not_enough_online": online,
                }),
            ),
            cron::UpdateResult::AllOk => renderer.render("cron", json!({})),
        }?,
    )
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        index,
        list,
        list_formfail,
        prepare_action,
        run_action,
        cron_route
    ]
}
