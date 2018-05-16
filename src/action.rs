use rocket::request::FromFormValue;
use rocket::http::RawStr;

use ring::{digest, hmac, error};

use rmp_serde::to_vec as serialize_to_vec;

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

#[derive(Serialize, Deserialize, FromForm, Clone)]
pub struct Action {
    pub node: String,
    pub email: String,
    pub op: Operation,
}

#[derive(Serialize, Deserialize)]
pub struct SignedAction {
    action: Action,
    signature: Box<[u8]>,
}

impl Action {
    fn compute_signature(&self, key: &[u8]) -> hmac::Signature {
        let key = hmac::SigningKey::new(&digest::SHA256, key);
        let buf = serialize_to_vec(self).expect("failed to encode Action");
        let signature = hmac::sign(&key, buf.as_slice());
        signature
    }

    fn verify_signature(&self, key: &[u8], signature: &[u8]) -> Result<(), error::Unspecified> {
        let key = hmac::VerificationKey::new(&digest::SHA256, key);
        let buf = serialize_to_vec(self).expect("failed to encode Action");
        hmac::verify(&key, buf.as_slice(), signature)
    }

    pub fn sign(self, key: &[u8]) -> SignedAction {
        let signature = self.compute_signature(key);
        let signature = signature.as_ref().to_vec().into_boxed_slice();
        SignedAction { action: self, signature }
    }
}

impl SignedAction {
    pub fn verify(self, key: &[u8]) -> Result<Action, error::Unspecified> {
        // Using a match to make it really clear we don't return the action in case of failure
        match self.action.verify_signature(key, &*self.signature) {
            Ok(_) => Ok(self.action),
            Err(e) => Err(e),
        }
    }
}
