use ring::{digest, hmac, error};
use rmps::to_vec;

#[derive(Serialize)]
pub struct Action {
    node: String,
    mail: String,
    add: bool, // false = remove
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
