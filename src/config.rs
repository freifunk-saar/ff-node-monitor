use rocket;
use toml;
use serde::Deserialize;
use serde::de::IntoDeserializer;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub instance_name: String,
    pub root_url: String,
    pub email_from: String,
}

impl Config {
    /// Create a `Config` instance from a rocket config table
    pub fn new(table: &rocket::config::Table) -> Self {
        let val = toml::value::Value::Table(table.clone());
        Self::deserialize(val.into_deserializer())
            .expect("[ff-node-monitor] config table has missing or extra value")
    }
}
