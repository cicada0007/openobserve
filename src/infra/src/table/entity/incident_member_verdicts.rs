// Copyright 2026 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! `SeaORM` Entity for incident_member_verdicts: one membership verdict per (incident, alert).

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, serde::Serialize, serde::Deserialize)]
#[sea_orm(table_name = "incident_member_verdicts")]
pub struct Model {
    pub org_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub incident_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub alert_id: String,
    /// related, unrelated or unsure
    pub verdict: String,
    #[sea_orm(column_type = "Text")]
    pub reason: String,
    /// JSON string `{signals:[{name,state,value,line}], rule?}`, parsed by serde.
    #[sea_orm(column_type = "Text")]
    pub evidence: String,
    /// rule, ai, user or memory
    pub decided_by: String,
    /// The deciding signal of a rule verdict; for memory, how the remembered verdict was inferred.
    pub signal: Option<String>,
    pub decided_by_user: Option<String>,
    pub model: Option<String>,
    pub decided_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
