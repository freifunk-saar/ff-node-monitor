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

#[derive(Serialize, Deserialize)]
struct Nodes {
    version: usize,
    nodes: Vec<Value>,
    timestamp: DateTime<Utc>,
}

/// Fetch the latest nodelist, update node state and send out emails
pub fn update_nodes(db: &PgConnection, config: &config::Config) -> Result<(), Error> {
    let nodes = reqwest::get(config.urls.nodes_url.as_str())?;
    let nodes: Nodes = serde_json::from_reader(nodes)?;

    if nodes.version != 2 {
        Err(NodeListError::UnsupportedVersion { version: nodes.version })?;
    }

    println!("{}", nodes.nodes.len());

    Ok(())
}
