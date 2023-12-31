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

use std::collections::HashMap;

use anyhow::{bail, Result};
use diesel::prelude::*;
use serde_json::{self, json};
use thiserror::Error;

use rocket::uri;

use crate::db::DbConn;
use crate::email::EmailAddress;
use crate::models;
use crate::routes;
use crate::schema::*;
use crate::util::Ctx;

#[derive(Debug, Error)]
enum NodeListError {
    #[error("got unsupported version number {version}")]
    UnsupportedVersion { version: usize },
}

mod json {
    use chrono::{DateTime, Utc};
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    pub(crate) struct NodeInfo {
        pub(crate) node_id: Option<String>,
        pub(crate) hostname: Option<String>,
    }

    #[derive(Deserialize, Debug)]
    pub(crate) struct Flags {
        pub(crate) online: bool,
    }

    #[derive(Deserialize, Debug)]
    #[allow(unused)] // we don't actually need all these fields
    pub(crate) struct Statistics {
        pub(crate) memory_usage: Option<f64>,
        pub(crate) rootfs_usage: Option<f64>,
        pub(crate) loadavg: Option<f64>,
    }

    #[derive(Deserialize, Debug)]
    #[allow(unused)] // we don't actually need all these fields
    pub(crate) struct Node {
        pub(crate) nodeinfo: NodeInfo,
        pub(crate) flags: Flags,
        pub(crate) statistics: Statistics,
        pub(crate) lastseen: DateTime<Utc>,
        pub(crate) firstseen: DateTime<Utc>,
    }

    #[derive(Deserialize, Debug)]
    #[allow(unused)] // we don't actually need all these fields
    pub(crate) struct Nodes {
        pub(crate) version: usize,
        pub(crate) nodes: Vec<Node>,
        pub(crate) timestamp: DateTime<Utc>,
    }
}

// Just the data about the node (the RHS of the HashMap)
#[derive(Clone, PartialEq, Eq, serde::Deserialize)]
struct NodeData {
    name: String,
    online: bool,
}

// From a JSON node, extract node ID and other information
fn json_to_node_data(node: json::Node) -> Option<(String, NodeData)> {
    let node_data = NodeData {
        name: node.nodeinfo.hostname?,
        online: node.flags.online,
    };
    Some((node.nodeinfo.node_id?, node_data))
}

fn model_to_node_data(node: models::NodeQuery) -> (String, NodeData) {
    let node_data = NodeData {
        name: node.name,
        online: node.online,
    };
    (node.id, node_data)
}

impl NodeData {
    fn into_model(self, id: String) -> models::NodeQuery {
        models::NodeQuery {
            id,
            name: self.name,
            online: self.online,
        }
    }
}

#[must_use]
pub enum UpdateResult {
    AllOk,
    NotEnoughOnline(usize),
}

/// Fetch the latest nodelist, update node state and send out emails
impl<'r> Ctx<'r> {
    pub async fn update_nodes(&self, db: &DbConn) -> Result<UpdateResult> {
        let config = self.config();
        let cur_nodes: json::Nodes = reqwest::get(config.urls.nodes.clone())
            .await?
            .json()
            .await?;

        if cur_nodes.version != 2 {
            bail!(NodeListError::UnsupportedVersion {
                version: cur_nodes.version
            });
        }

        // Build node HashMap: map node ID to name and online state
        let mut cur_nodes_map: HashMap<String, NodeData> = HashMap::new();
        for cur_node in cur_nodes.nodes.into_iter() {
            if let Some((id, data)) = json_to_node_data(cur_node) {
                cur_nodes_map.insert(id, data);
            }
        }

        // Stop here if nearly all nodes are offline
        let online_nodes = cur_nodes_map.values().filter(|data| data.online).count();
        if online_nodes < config.ui.min_online_nodes.unwrap_or(0) {
            return Ok(UpdateResult::NotEnoughOnline(online_nodes));
        }

        // Compute which nodes changed their state, also update node names in DB
        let changed: Vec<(String, NodeData)> = db
            .run(move |db| {
                db.transaction::<_, anyhow::Error, _>(|db| {
                    {
                        let mut changed = Vec::new();

                        // Go over every node in the database
                        let db_nodes = nodes::table.load::<models::NodeQuery>(db)?;
                        for db_node in db_nodes.into_iter() {
                            let (id, db_data) = model_to_node_data(db_node);
                            if let Some(cur_data) = cur_nodes_map.remove(&id) {
                                // We already know this node.
                                // Did it change?
                                if cur_data != db_data {
                                    // Update in database
                                    diesel::update(nodes::table.find(id.as_str()))
                                        .set((
                                            nodes::name.eq(cur_data.name.as_str()),
                                            nodes::online.eq(cur_data.online),
                                        ))
                                        .execute(db)?;
                                }
                                // Did its online status change?
                                if cur_data.online != db_data.online {
                                    changed.push((id, cur_data));
                                }
                            } else {
                                // The node is in the DB but does not exist any more.
                                diesel::delete(nodes::table.find(id.as_str())).execute(db)?;
                                if db_data.online {
                                    // The node was online, so it being gone is a change to offline
                                    changed.push((
                                        id,
                                        NodeData {
                                            online: false,
                                            ..db_data
                                        },
                                    ));
                                }
                            }
                        }

                        // Go over nodes remaining in the hash map -- they are not in the DB
                        for (id, cur_data) in cur_nodes_map.into_iter() {
                            // Insert into DB
                            diesel::insert_into(nodes::table)
                                .values(&models::Node {
                                    id: id.as_str(),
                                    name: cur_data.name.as_str(),
                                    online: cur_data.online,
                                })
                                .execute(db)?;
                            if cur_data.online {
                                // The node is online, so it appearing is a change from the implicit offline
                                // it was in when it did not exist.
                                changed.push((id, cur_data));
                            }
                        }

                        Ok(changed)
                    }
                })
            })
            .await?;

        // Send out notifications (not in the transaction as we don't really care here -- also
        // we have an external side-effect, the email, which we cannot roll back anyway)
        for (id, cur_data) in changed.into_iter() {
            // See who monitors this node
            let watchers = db
                .run({
                    let id = id.clone();
                    move |db| {
                        monitors::table
                            .filter(monitors::id.eq(id.as_str()))
                            .load::<models::MonitorQuery>(db)
                    }
                })
                .await?;
            // Send them email
            let node = cur_data.into_model(id);
            for watcher in watchers.iter() {
                // Generate email text
                let email = EmailAddress::new(watcher.email.clone()).unwrap();
                let list_url = config.urls.absolute(uri!(routes::list(email = &email)));
                // Build and send email
                self.email(
                    "notification",
                    json!({
                        "node": node,
                        "list_url": list_url.as_str(),
                    }),
                    watcher.email.as_str(),
                )
                .await?;
            }
        }

        Ok(UpdateResult::AllOk)
    }
}
