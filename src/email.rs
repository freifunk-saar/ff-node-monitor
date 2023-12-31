use std::{ops::Deref, str::FromStr as _};

use anyhow::{bail, Result};
use lettre::{
    message::{header::ContentType, Mailbox},
    Address, AsyncSmtpTransport, AsyncTransport as _, Message, Tokio1Executor,
};
use serde::{Deserialize, Serialize};

use rocket::{
    form::{self, FromFormField},
    UriDisplayQuery,
};
use rocket_dyn_templates::Template;

use crate::config::Config;
use crate::util::Ctx;

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
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'r> Ctx<'r> {
    /// Build an email from a template and send it
    pub async fn email(
        &self,
        email_template: &'static str,
        vals: serde_json::Value,
        to: &str,
    ) -> Result<()> {
        let config = self.state::<Config>().unwrap();
        let email_text = Template::show(self, email_template, config.template_vals(vals)?).unwrap();
        let email_parts: Vec<&str> = email_text.splitn(3, '\n').collect();
        let (email_from, email_subject, email_body) =
            (email_parts[0], email_parts[1], email_parts[2]);

        // Build email
        let message = Message::builder()
            .from(Mailbox::new(
                Some(email_from.to_owned()),
                config.ui.email_from.clone(),
            ))
            .to(Address::from_str(to)?.into())
            .subject(email_subject)
            .header(ContentType::TEXT_PLAIN)
            .body(email_body.to_owned())
            .unwrap();

        // Send email
        let smtp_host = config.secrets.get_smtp_host();
        let mailer = if smtp_host == "localhost" {
            AsyncSmtpTransport::unencrypted_localhost() // always port 25
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(smtp_host)?
                .port(25)
                .build()
        };
        let r = mailer.send(message).await?;
        if !r.is_positive() {
            bail!(
                "sending email failed:\n{}",
                r.first_line().unwrap_or("<no message>")
            );
        }
        Ok(())
    }
}
