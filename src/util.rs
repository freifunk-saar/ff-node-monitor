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

use rocket::{
    Request,
    Outcome,
    State,
    request::{Outcome as ReqOutcome, FromRequest, FromFormValue},
    response::Responder,
    http::{Status, RawStr},
    UriDisplayQuery,
};
use rocket_contrib::templates::Template;

use anyhow::{Result, bail};
use thiserror::Error;
use mail::{Mail, Email, smtp, headers, HeaderTryFrom, default_impl::simple_context};
use serde::{Serialize, Deserialize};
use futures::Future;

use crate::config::Config;

use std::ops::Deref;

/// Module for serde "with" to use hex encoding to byte arrays
pub mod hex_signing_key {
    use hex;
    use serde::{Deserializer, Deserialize, de::Error};
    use ring::{digest, hmac};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<hmac::SigningKey, D::Error>
        where D: Deserializer<'de>
    {
         let bytes = hex::decode(String::deserialize(deserializer)?).map_err(Error::custom)?;
         Ok(hmac::SigningKey::new(&digest::SHA256, bytes.as_slice()))
    }
}

/// Type for email addresses in Rocket forms
#[derive(Clone, Serialize, Deserialize, UriDisplayQuery)]
pub struct EmailAddress(String);

impl EmailAddress {
    pub fn new(s: String) -> Result<EmailAddress> {
        let email_parts : Vec<&str> = s.split('@').collect();
        if email_parts.len() != 2 {
            bail!("Too many or two few @");
        }
        if email_parts[0].is_empty() {
            bail!("User part is empty");
        }
        if email_parts[1].find('.').is_none() {
            bail!("Domain part must contain .");
        }
        Ok(EmailAddress(s))
    }
}

impl<'v> FromFormValue<'v> for EmailAddress {
    type Error = anyhow::Error;

    fn from_form_value(v: &'v RawStr) -> Result<EmailAddress> {
        let s = v.url_decode()?;
        EmailAddress::new(s)
    }
}

impl Deref for EmailAddress {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Horribly hacky hack to get access to the Request, and then a template's body, for building emails
pub struct EmailSender<'a, 'r> {
    request: &'a Request<'r>,
    config: &'a Config,
    mail_ctx: &'a simple_context::Context,
}

#[derive(Debug, Error)]
enum ResponderError {
    #[error("responder failed with status {status}")]
    RenderFailure {
        status: Status,
    },
    #[error("couldn't find a body")]
    NoBody,
}

#[derive(Debug, Error)]
enum EmailError {
    #[error("{0}")]
    ComponentCreation(mail::error::ComponentCreationError),
    #[error("{0}")]
    MailSend(mail::error::MailSendError),
}

impl<'a, 'r> FromRequest<'a, 'r> for EmailSender<'a, 'r> {
    type Error = ();
    fn from_request(request: &'a Request<'r>) -> ReqOutcome<Self, Self::Error> {
        let config = request.guard::<State<Config>>()?.inner();
        let mail_ctx = request.guard::<State<simple_context::Context>>()?.inner();
        Outcome::Success(EmailSender { request, config, mail_ctx })
    }
}

impl<'a, 'r> EmailSender<'a, 'r> {
    fn responder_body<'re>(&self, responder: impl Responder<'re>) -> Result<String> {
        let mut resp = responder.respond_to(self.request)
            .map_err(|status| ResponderError::RenderFailure { status })?;
        Ok(resp.body_string().ok_or(ResponderError::NoBody)?)
    }

    /// Build an email from a template and send it
    pub fn email(&self, email_template: Template, to: &str) -> Result<()> {
        let email_text = self.responder_body(email_template)?;
        let email_parts : Vec<&str> = email_text.splitn(4, '\n').collect();
        let (empty, email_from, email_subject, email_body) = (email_parts[0], email_parts[1], email_parts[2], email_parts[3]);
        assert!(empty.is_empty(), "The first line of the email template must be empty");

        // Build email
        let from = Email::try_from(self.config.ui.email_from.as_str())
            .map_err(EmailError::ComponentCreation)?;
        let mut mail = Mail::plain_text(email_body, self.mail_ctx);
        mail.insert_headers(headers! {
            headers::_From: [(email_from, from)],
            headers::_To: [to],
            headers::Subject: email_subject
        }.map_err(EmailError::ComponentCreation)?);

        // Send email
        let config = if self.config.secrets.get_smtp_host() == "localhost" {
            smtp::ConnectionConfig::builder_local_unencrypted().port(25).build()
        } else {
            let smtp_host = self.config.secrets.get_smtp_host();
            smtp::ConnectionConfig::builder_with_port(smtp_host.parse()?, 25)?.build()
        };
        Ok(smtp::send(mail.into(), config, self.mail_ctx.clone()).wait().map_err(EmailError::MailSend)?)
    }
}
