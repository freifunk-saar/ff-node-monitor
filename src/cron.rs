use diesel::prelude::*;
use diesel::pg::PgConnection;
use failure::Error;
use serde_json::{self, Value};
use chrono::{DateTime, Utc};

use config;
use reqwest;

#[derive(Debug, Fail)]
enum NodeListError {
    #[fail(display = "got unsupported version number {}", version)]
    UnsupportedVersion {
        version: usize,
    },
}

#[derive(Deserialize, Debug)]
struct NodeInfo {
    node_id: String,
    hostname: String,
}

#[derive(Deserialize, Debug)]
struct Flags {
    online: bool,
}

#[derive(Deserialize, Debug)]
struct Statistics {
    memory_usage: Option<f64>,
    rootfs_usage: Option<f64>,
    loadavg: Option<f64>,
    clients: Option<usize>,
}

#[derive(Deserialize, Debug)]
struct Node {
    nodeinfo: NodeInfo,
    flags: Flags,
    statistics: Statistics,
    lastseen: DateTime<Utc>,
    firstseen: DateTime<Utc>,
}

#[derive(Deserialize, Debug)]
struct Nodes {
    version: usize,
    nodes: Vec<Node>,
    timestamp: DateTime<Utc>,
}

/// Fetch the latest nodelist, update node state and send out emails
pub fn update_nodes(db: &PgConnection, config: &config::Config) -> Result<(), Error> {
    let nodes = reqwest::get(config.urls.nodes_url.clone())?;
    let nodes: Nodes = serde_json::from_reader(nodes)?;

    if nodes.version != 2 {
        Err(NodeListError::UnsupportedVersion { version: nodes.version })?;
    }

    println!("{:#?}", nodes.nodes);

    Ok(())
}
