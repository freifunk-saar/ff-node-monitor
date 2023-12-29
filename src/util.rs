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

use std::ops::Deref;

use rocket::{
    form::{self, FromFormField},
    request::{self, FromRequest, Outcome},
    Request, State, UriDisplayQuery,
};
use rocket_dyn_templates::Template;

use anyhow::Result;
use futures::Future;
use mail::{default_impl::simple_context, headers, smtp, Email, HeaderTryFrom, Mail};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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

/// Type for email addresses in Rocket forms
#[derive(Clone, Serialize, Deserialize, UriDisplayQuery)]
pub struct EmailAddress(String);

impl EmailAddress {
    pub fn new<'e>(s: String) -> form::Result<'e, EmailAddress> {
        let email_parts: Vec<&str> = s.split('@').collect();
        if email_parts.len() != 2 {
            return Err(rocket::form::Error::validation("invalid credit card number").into());
        }
        if email_parts[0].is_empty() {
            return Err(rocket::form::Error::validation("User part is empty").into());
        }
        if email_parts[1].find('.').is_none() {
            return Err(rocket::form::Error::validation("Domain part must contain .").into());
        }
        Ok(EmailAddress(s))
    }
}

#[rocket::async_trait]
impl<'r> FromFormField<'r> for EmailAddress {
    fn from_value(field: form::ValueField<'r>) -> form::Result<'r, Self> {
        // `new` does address validation
        EmailAddress::new(String::from_value(field)?)
    }
}

impl Deref for EmailAddress {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Horribly hacky hack to get access to the Request, and then a template's body, for building emails
pub struct EmailSender<'a> {
    rocket: &'a rocket::Rocket<rocket::Orbit>,
    config: &'a Config,
    mail_ctx: &'a simple_context::Context,
}

#[derive(Debug, Error)]
enum EmailError {
    #[error("{0}")]
    ComponentCreation(mail::error::ComponentCreationError),
    #[error("{0}")]
    MailSend(mail::error::MailSendError),
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for EmailSender<'r> {
    type Error = ();
    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let config = request
            .guard::<&State<Config>>()
            .await
            .expect("config")
            .inner();
        let mail_ctx = request
            .guard::<&State<simple_context::Context>>()
            .await
            .expect("mail_ctx")
            .inner();
        Outcome::Success(EmailSender {
            rocket: request.rocket(),
            config,
            mail_ctx,
        })
    }
}

impl<'r> EmailSender<'r> {
    /// Build an email from a template and send it
    pub async fn email(
        &self,
        email_template: &'static str,
        vals: serde_json::Value,
        to: &str,
    ) -> Result<()> {
        let email_text = Template::show(
            self.rocket,
            email_template,
            self.config.template_vals(vals)?,
        )
        .unwrap();
        //let email_text = self.responder_body(email_template).await?;
        let email_parts: Vec<&str> = email_text.splitn(3, '\n').collect();
        let (email_from, email_subject, email_body) =
            (email_parts[0], email_parts[1], email_parts[2]);

        // Build email
        let from = <Email as HeaderTryFrom<_>>::try_from(self.config.ui.email_from.as_str())
            .map_err(EmailError::ComponentCreation)?;
        let mut mail = Mail::plain_text(email_body, self.mail_ctx);
        mail.insert_headers(
            headers! {
                headers::_From: [(email_from, from)],
                headers::_To: [to],
                headers::Subject: email_subject
            }
            .map_err(EmailError::ComponentCreation)?,
        );

        // Send email
        let config = if self.config.secrets.get_smtp_host() == "localhost" {
            smtp::ConnectionConfig::builder_local_unencrypted()
                .port(25)
                .build()
        } else {
            let smtp_host = self.config.secrets.get_smtp_host();
            smtp::ConnectionConfig::builder_with_port(smtp_host.parse()?, 25)?.build()
        };
        Ok(smtp::send(mail.into(), config, self.mail_ctx.clone())
            .wait()
            .map_err(EmailError::MailSend)?)
    }
}
