use rocket::request::FromFormValue;
use rocket::http::RawStr;

use diesel::prelude::*;
use diesel;
use diesel::result::{Error as DieselError, DatabaseErrorKind};
use ring::{hmac, error};
use failure::Error;
use rmp_serde::to_vec as serialize_to_vec;

use schema::*;
use models::*;

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

    pub fn run(&self, db: &PgConnection) -> Result<bool, Error> {
        let m = Monitor { id: self.node.as_str(), email: self.email.as_str() };
        Ok(match self.op {
            Operation::Add => {
                db.transaction::<_, Error, _>(|| {
                    // Check if the node ID even exists
                    let node = nodes::table
                        .find(self.node.as_str())
                        .first::<NodeQuery>(db).optional()?;
                    if node.is_none() {
                        return Ok(false);
                    }
                    // Add it
                    let r = diesel::insert_into(monitors::table)
                        .values(&m)
                        .execute(db);
                    // Handle UniqueViolation gracefully
                    Ok(match r {
                        Ok(_) => true,
                        Err(DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => false,
                        Err(e) => bail!(e),
                    })
                })?
            }
            Operation::Remove => {
                let num_deleted = diesel::delete(&m).execute(db)?;
                num_deleted > 0
            }
        })
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
