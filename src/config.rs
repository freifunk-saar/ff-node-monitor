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

use rocket::{self, State, Outcome, http::uri};
use rocket::request::{self, Request, FromRequest};
use rocket::fairing::{Fairing, AdHoc};
use rocket_contrib::templates::Template;

use toml;
use url::Url;
use url_serde;
use serde::{Deserialize, Serialize, de::IntoDeserializer};
use serde_json::{self, json};
use ring::hmac;
use anyhow::{Result, bail};
use mail::{Email, HeaderTryFrom, default_impl::simple_context};
use uuid::Uuid;

use std::borrow::Cow;

use crate::util;

#[derive(Serialize, Deserialize)]
pub struct Ui {
    pub instance_name: String,
    pub instance_article_dative: String,
    pub email_from: String,
    pub min_online_nodes: Option<usize>,
}

#[derive(Serialize, Deserialize)]
pub struct Urls {
    #[serde(with = "url_serde")]
    pub root: Url,
    #[serde(with = "url_serde")]
    pub nodes: Url,
    #[serde(with = "url_serde")]
    pub sources: Url,
    pub stylesheet: Option<String>,
}

impl Urls {
    pub fn absolute(&self, origin: uri::Origin) -> String {
        format!("{}{}", self.root.as_str().trim_end_matches('/'), origin)
    }
}

#[derive(Deserialize)]
pub struct Secrets {
    pub smtp_host: Option<String>,
    #[serde(with = "util::hex_signing_key")]
    pub action_signing_key: hmac::SigningKey,
}

impl Secrets {
    /// Getters for default values
    pub fn get_smtp_host(&self) -> &str {
        self.smtp_host.as_ref().map(String::as_str).unwrap_or("localhost")
    }
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
    AdHoc::on_attach("Parse application configuration", move |rocket| {
        let config = {
            let config_table = rocket.config().get_table(section)
                .unwrap_or_else(|_| panic!("[{}] table in Rocket.toml missing or not a table", section));
            Config::new(config_table)
        };
        let mail_ctx = {
            let from = Email::try_from(config.ui.email_from.as_str()).expect("`email_from` is not a valid email address");
            let unique_part = Uuid::new_v4().to_string().parse().unwrap();
            simple_context::new(from.domain, unique_part).unwrap()
        };
        Ok(rocket.manage(config).manage(mail_ctx))
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
    ) -> Result<Template> {
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
