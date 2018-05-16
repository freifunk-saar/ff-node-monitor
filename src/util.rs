use rocket::{
    Request as Req,
    Outcome,
    request::{Outcome as ReqOutcome, FromRequest},
    response::Responder,
    http::Status,
};

use failure::Error;

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
