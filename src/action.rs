use rocket::request::FromFormValue;
use rocket::http::RawStr;

use ring::{hmac, error};

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
    fn compute_signature(&self, key: &hmac::SigningKey) -> hmac::Signature {
        let buf = serialize_to_vec(self).expect("failed to encode Action");
        let signature = hmac::sign(&key, buf.as_slice());
        signature
    }

    fn verify_signature(
        &self,
        key: &hmac::SigningKey,
        signature: &[u8],
    ) -> Result<(), error::Unspecified> {
        let buf = serialize_to_vec(self).expect("failed to encode Action");
        hmac::verify_with_own_key(&key, buf.as_slice(), signature)
    }

    pub fn sign(self, key: &hmac::SigningKey) -> SignedAction {
        let signature = self.compute_signature(key);
        let signature = signature.as_ref().to_vec().into_boxed_slice();
        SignedAction { action: self, signature }
    }
}

impl SignedAction {
    pub fn verify(self, key: &hmac::SigningKey) -> Result<Action, error::Unspecified> {
        // Using a match to make it really clear we don't return the action in case of failure
        match self.action.verify_signature(key, &*self.signature) {
            Ok(_) => Ok(self.action),
            Err(e) => Err(e),
        }
    }
}
