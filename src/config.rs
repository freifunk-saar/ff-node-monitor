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

use rocket::{self, State, Outcome};
use rocket::request::{self, Request, FromRequest};
use rocket::fairing::{Fairing, AdHoc};
use rocket_contrib::Template;

use toml;
use url::Url;
use url_serde;
use serde::Deserialize;
use serde::de::IntoDeserializer;
use ring::hmac;
use serde_json;
use failure::Error;

use std::borrow::Cow;

use db_conn;
use util;

#[derive(Serialize, Deserialize)]
pub struct Ui {
    pub instance_name: String,
    pub email_from: String,
}

#[derive(Serialize, Deserialize)]
pub struct Urls {
    #[serde(with = "url_serde")]
    pub root: Url,
    #[serde(with = "url_serde")]
    pub nodes: Url,
}

#[derive(Deserialize)]
pub struct Secrets {
    pub postgres_url: String,
    #[serde(with = "util::hex_signing_key")]
    pub action_signing_key: hmac::SigningKey,
}

#[derive(Deserialize)]
pub struct Config {
    pub ui: Ui,
    pub secrets: Secrets,
    pub urls: Urls,
}

impl Config {
    /// Create a `Config` instance from a rocket config table
    pub fn new(table: &rocket::config::Table) -> Self {
        let val = toml::value::Value::Table(table.clone());
        Self::deserialize(val.into_deserializer())
            .expect("app config table has missing or extra value")
    }
}

pub fn fairing(section: &'static str) -> impl Fairing {
    AdHoc::on_attach(move |rocket| {
        let config = {
            let config_table = rocket.config().get_table(section)
                .expect(format!("[{}] table in Rocket.toml missing or not a table", section).as_str());
            Config::new(config_table)
        };
        Ok(rocket
            .manage(db_conn::init_db_pool(config.secrets.postgres_url.as_str()))
            .manage(config))
    })
}

/// A request guard that makes the config available to all templates
pub struct Renderer<'a>(&'a Config);

impl<'a, 'r> FromRequest<'a, 'r> for Renderer<'a> {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, ()> {
        Outcome::Success(Renderer(request.guard::<State<Config>>()?.inner()))
    }
}

impl<'a> Renderer<'a> {
    pub fn render(
        &self,
        name: impl Into<Cow<'static, str>>,
        mut context: serde_json::Value
    ) -> Result<Template, Error> {
        if let Some(obj) = context.as_object_mut() {
            let old = obj.insert("config".to_string(), json!({
                "ui": self.0.ui,
                "urls": self.0.urls,
            }));
            if old.is_some() {
                bail!("Someone else already put a config here")
            }
        } else {
            bail!("The context must be a JSON object")
        }
        Ok(Template::render(name, context))
    }
}
