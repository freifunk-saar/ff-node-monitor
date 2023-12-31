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

use std::borrow::Cow;
use std::ops::Deref;

use anyhow::Result;

use rocket::{
    request::{self, FromRequest, Outcome},
    Request,
};
use rocket_dyn_templates::Template;

use crate::config::Config;

/// Module for serde "with" to use hex encoding to byte arrays
pub mod hex_signing_key {
    use hex;
    use ring::hmac;
    use serde::{de::Error, Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<hmac::Key, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = hex::decode(String::deserialize(deserializer)?).map_err(Error::custom)?;
        Ok(hmac::Key::new(hmac::HMAC_SHA256, bytes.as_slice()))
    }
}

/// A request guard to get access to the rocket.
pub struct Ctx<'r>(&'r rocket::Rocket<rocket::Orbit>);

impl Deref for Ctx<'_> {
    type Target = rocket::Rocket<rocket::Orbit>;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Ctx<'r> {
    type Error = ();
    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        Outcome::Success(Ctx(request.rocket()))
    }
}

impl Ctx<'_> {
    pub fn config(&self) -> &Config {
        self.state::<Config>().unwrap()
    }

    pub fn template(
        &self,
        name: impl Into<Cow<'static, str>>,
        vals: serde_json::Value,
    ) -> Result<Template> {
        Ok(Template::render(name, self.config().template_vals(vals)?))
    }
}
