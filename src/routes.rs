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
use diesel::prelude::*;
use rmp_serde::from_slice as deserialize_from_slice;
use rmp_serde::to_vec as serialize_to_vec;
use serde_json::json;

use rocket::{form::Form, response, State};
use rocket::{get, post, routes, uri, Request};
use rocket_dyn_templates::Template;

use crate::action::*;
use crate::config::Config;
use crate::cron;
use crate::db::DbConn;
use crate::email::EmailAddress;
use crate::models::*;
use crate::util::Ctx;

const BASE64_ENGINE: base64::engine::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// Custom error type to allow using `?` below.
struct Error(anyhow::Error);

impl<'r> response::Responder<'r, 'static> for Error {
    fn respond_to(self, r: &'r Request<'_>) -> response::Result<'static> {
        response::Debug(self.0).respond_to(r)
    }
}

impl<T> From<T> for Error
where
    anyhow::Error: From<T>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

type Result<T> = std::result::Result<T, Error>;

#[get("/")]
fn index(ctx: Ctx<'_>) -> Result<Template> {
    Ok(ctx.template("index", json!({}))?)
}

#[get("/list?<email>")]
async fn list(email: EmailAddress, ctx: Ctx<'_>, db: DbConn) -> Result<Template> {
    use crate::schema::*;

    let vars = db
        .run_transaction(move |db| {
            let watched_nodes = monitors::table
                .filter(monitors::email.eq(&*email))
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

    Ok(ctx.template("list", vars)?)
}

#[get("/list")]
fn list_formfail(ctx: Ctx<'_>) -> Result<Template> {
    Ok(ctx.template("list_error", json!({}))?)
}

#[post("/prepare_action", data = "<action>")]
async fn prepare_action(
    action: Form<Action>,
    config: &State<Config>,
    ctx: Ctx<'_>,
    db: DbConn,
) -> Result<Template> {
    use crate::schema::*;

    let action = action.into_inner();

    // obtain bytes for signed action payload
    let signed_action = action.clone().sign(&config.secrets.action_signing_key);
    let signed_action = serialize_to_vec(&signed_action)?;
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
        .await?;
    let node_name = match node {
        Some(node) => node.name,
        None if action.op == Operation::Remove =>
        // Allow removing dead nodes
        {
            action.node.clone()
        }
        None => {
            // Trying to add a non-existing node. Stop this.
            return Ok(ctx.template(
                "prepare_action_error",
                json!({
                    "action": action,
                    "list_url": list_url.as_str(),
                }),
            )?);
        }
    };

    // Build and send email
    ctx.email(
        "confirm_action",
        json!({
            "action": action,
            "node_name": node_name,
            "action_url": action_url.as_str(),
            "list_url": list_url.as_str(),
        }),
        &action.email,
    )
    .await?;

    // Render
    Ok(ctx.template(
        "prepare_action",
        json!({
            "action": action,
            "node_name": node_name,
            "list_url": list_url,
        }),
    )?)
}

#[get("/run_action?<signed_action>")]
async fn run_action(signed_action: String, db: DbConn, ctx: Ctx<'_>) -> Result<Template> {
    // Determine and verify action
    let action: Result<Action> = (|| {
        let signed_action = BASE64_ENGINE.decode(signed_action.as_str())?;
        let signed_action: SignedAction = deserialize_from_slice(signed_action.as_slice())?;
        Ok(signed_action
            .verify(&ctx.config().secrets.action_signing_key)
            .map_err(|_| anyhow::anyhow!("signature verification failed"))?)
    })();
    let action = match action {
        Ok(a) => a,
        Err(_) => return Ok(ctx.template("run_action_error", json!({}))?),
    };

    // Execute action
    let success = action.run(&db).await?;

    // Render
    let list_url = ctx
        .config()
        .urls
        .absolute(uri!(list(email = &action.email)));
    Ok(ctx.template(
        "run_action",
        json!({
            "action": action,
            "list_url": list_url,
            "success": success,
        }),
    )?)
}

#[get("/cron")]
async fn cron_route(db: DbConn, ctx: Ctx<'_>) -> Result<Template> {
    Ok(match ctx.update_nodes(&db).await? {
        cron::UpdateResult::NotEnoughOnline(online) => ctx.template(
            "cron_error",
            json!({
                "not_enough_online": online,
            }),
        ),
        cron::UpdateResult::AllOk => ctx.template("cron", json!({})),
    }?)
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
