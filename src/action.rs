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

use rocket::request::FromFormValue;
use rocket::http::RawStr;

use diesel::prelude::*;
use diesel;
use diesel::result::{Error as DieselError, DatabaseErrorKind};
use ring::{hmac, error};
use failure::{Error, bail};
use rmp_serde::to_vec as serialize_to_vec;
use serde_derive::{Deserialize, Serialize};

use crate::schema::*;
use crate::models::*;

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
        hmac::sign(&key, buf.as_slice())
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
        db.transaction::<_, Error, _>(|| {
            Ok(match self.op {
                Operation::Add => {
                    // Add node.  We are fine if it does not exist.
                    let r = diesel::insert_into(monitors::table)
                        .values(&m)
                        .execute(db);
                    // Handle UniqueViolation gracefully
                    match r {
                        Ok(_) => true,
                        Err(DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => false,
                        Err(e) => bail!(e),
                    }
                }
                Operation::Remove => {
                    let num_deleted = diesel::delete(&m).execute(db)?;
                    num_deleted > 0
                }
            })
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
