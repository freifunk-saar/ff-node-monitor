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

use rocket::form::FromFormField;
use rocket::FromForm;

use anyhow::{bail, Result};
use diesel;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use ring::{error, hmac};
use rmp_serde::to_vec as serialize_to_vec;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::models::*;
use crate::schema::*;
use crate::util::EmailAddress;

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug, Copy, Clone, FromFormField)]
#[repr(u8)]
pub enum Operation {
    Add = 1,
    Remove = 0,
}

#[derive(Serialize, Deserialize, FromForm, Clone)]
pub struct Action {
    pub node: String,
    pub email: EmailAddress,
    pub op: Operation,
}

#[derive(Serialize, Deserialize)]
pub struct SignedAction {
    action: Action,
    signature: Box<[u8]>,
}

impl Action {
    fn compute_signature(&self, key: &hmac::Key) -> hmac::Tag {
        let buf = serialize_to_vec(self).expect("failed to encode Action");
        hmac::sign(&key, buf.as_slice())
    }

    fn verify_signature(
        &self,
        key: &hmac::Key,
        signature: &[u8],
    ) -> Result<(), error::Unspecified> {
        let buf = serialize_to_vec(self).expect("failed to encode Action");
        hmac::verify(&key, buf.as_slice(), signature)
    }

    pub fn sign(self, key: &hmac::Key) -> SignedAction {
        let signature = self.compute_signature(key);
        let signature = signature.as_ref().to_vec().into_boxed_slice();
        SignedAction {
            action: self,
            signature,
        }
    }

    pub async fn run(&self, db: &crate::DbConn) -> Result<bool> {
        let op = self.op;
        let node = self.node.clone();
        let email = self.email.clone();
        db.run(move |db| {
            let m = Monitor {
                id: node.as_str(),
                email: email.as_str(),
            };
            Ok(match op {
                Operation::Add => {
                    // Add node.  We are fine if it does not exist.
                    let r = diesel::insert_into(monitors::table).values(&m).execute(db);
                    // Handle UniqueViolation gracefully
                    match r {
                        Ok(_) => true,
                        Err(DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
                            false
                        }
                        Err(e) => bail!(e),
                    }
                }
                Operation::Remove => {
                    let num_deleted = diesel::delete(&m).execute(db)?;
                    num_deleted > 0
                }
            })
        })
        .await
    }
}

impl SignedAction {
    pub fn verify(self, key: &hmac::Key) -> Result<Action, error::Unspecified> {
        // Using a match to make it really clear we don't return the action in case of failure
        match self.action.verify_signature(key, &*self.signature) {
            Ok(_) => Ok(self.action),
            Err(e) => Err(e),
        }
    }
}
