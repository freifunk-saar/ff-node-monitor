use rocket::{
    Request as Req,
    Outcome,
    request::{Outcome as ReqOutcome, FromRequest},
    response::Responder,
    http::Status,
};

use failure::Error;

use std::ops;

/// Module for serde "with" to use hex encoding to byte arrays
pub mod hex_bytes {
    use hex;
    use serde::{Serializer, Deserializer, Deserialize, de::Error};

    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        serializer.serialize_str(hex::encode(bytes).as_str())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
        where D: Deserializer<'de>
    {
         Ok(hex::decode(String::deserialize(deserializer)?).map_err(|e| Error::custom(e))?)
    }
}

/// Horribly hacky hack to get access to the Request, and then a template's body
pub struct Request<'a, 'r: 'a>(&'a Req<'r>);

#[derive(Debug, Fail)]
enum ResponderError {
    #[fail(display = "responder failed with status {}", status)]
    RenderFailure {
        status: Status,
    },
    #[fail(display = "couldn't find a body")]
    NoBody,
}

impl<'a, 'r> Request<'a, 'r> {
    pub fn responder_body<'re, R: Responder<'re>>(&self, responder: R) -> Result<String, Error> {
        let mut resp = responder.respond_to(self.0)
            .map_err(|status| ResponderError::RenderFailure { status })?;
        Ok(resp.body_string().ok_or(ResponderError::NoBody)?)
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for Request<'a, 'r> {
    type Error = ();
    fn from_request(request: &'a Req<'r>) -> ReqOutcome<Self, Self::Error> {
        Outcome::Success(Request(request))
    }
}

impl<'a, 'r> ops::Deref for Request<'a, 'r> {
    type Target = Req<'r>;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}
