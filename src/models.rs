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

use serde::Serialize;

use crate::schema::*;

#[derive(Queryable, Serialize)]
pub struct MonitorQuery {
    pub id: String,
    pub email: String,
    pub initial_name: String,
}

#[derive(Insertable, Identifiable)]
#[primary_key(id, email)]
pub struct Monitor<'a> {
    pub id: &'a str,
    pub email: &'a str,
    pub initial_name: &'a str,
}

#[derive(Queryable, Serialize)]
pub struct NodeQuery {
    pub id: String,
    pub name: String,
    pub online: bool,
}

#[derive(Queryable, Serialize)]
pub struct MonitorNodeQuery {
    /// Details for what is being monitored
    pub monitor: MonitorQuery,
    /// If this monitors an existing node, information about that node
    pub node: Option<NodeQuery>,
}

#[derive(Insertable, Identifiable)]
#[primary_key(id)]
pub struct Node<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub online: bool,
}
