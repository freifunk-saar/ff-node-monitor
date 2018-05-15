use rocket::request::FromFormValue;
use rocket::http::RawStr;

use ring::{digest, hmac, error};
use rmps::to_vec;

enum_number!(Operation {
    Add = 1,
    Remove = 0,
});

impl<'v> FromFormValue<'v> for Operation {
    type Error = &'v RawStr;

    fn from_form_value(v: &'v RawStr) -> Result<Self, Self::Error> {
        match v.as_str() {
            "add" => Ok(Operation::Add),
            "remove" => Ok(Operation::Remove),
            _ => Err(v),
        }
    }
}

#[derive(Serialize, FromForm)]
pub struct Action {
    pub node: String,
    pub email: String,
    pub op: Operation,
}

impl Action {
    pub fn compute_signature(&self, key: &[u8]) -> hmac::Signature {
        let key = hmac::SigningKey::new(&digest::SHA256, key);
        let buf = to_vec(self).expect("failed to encode Action");
        let signature = hmac::sign(&key, buf.as_slice());
        signature
    }

    pub fn verify_signature(&self, key: &[u8], signature: &[u8]) -> Result<(), error::Unspecified> {
        let key = hmac::VerificationKey::new(&digest::SHA256, key);
        let buf = to_vec(self).expect("failed to encode Action");
        hmac::verify(&key, buf.as_slice(), signature)
    }
}
