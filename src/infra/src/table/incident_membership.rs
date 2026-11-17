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

//! Membership verdicts of incident members and the per-org memory of alert pairs.

use std::collections::HashMap;

use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    TransactionTrait,
    sea_query::{LockType, OnConflict},
};

use super::entity::{
    alert_incident_alerts, alert_incidents, alert_pair_verdicts, incident_member_verdicts,
    oncall_responses, service_version_deploys,
};
use crate::{
    db::{get_orm_client_ro, get_orm_client_rw},
    errors::{self, DbError, Error},
};

/// The pair verdict a move out stores: the alert does not rejoin incidents the opener starts.
pub const SEPARATE: &str = "separate";
/// The reason an AI verdict carries when no model answered.
pub const AI_UNAVAILABLE: &str = "AI check unavailable";

/// The unordered pair key: the lesser id first.
pub fn pair_key(x: &str, y: &str) -> (String, String) {
    if x <= y {
        (x.to_string(), y.to_string())
    } else {
        (y.to_string(), x.to_string())
    }
}

/// Writes a member verdict under the incident's row lock, which a move out and joins also take;
/// false when the alert left the incident or the incident resolved.
pub async fn put_member(row: incident_member_verdicts::Model) -> Result<bool, errors::Error> {
    let db = get_orm_client_rw().await;
    let txn = db.begin().await?;
    let incident = alert_incidents::Entity::find_by_id(row.incident_id.clone())
        .lock(LockType::Update)
        .one(&txn)
        .await?;
    let open = incident.is_some_and(|i| i.status != "resolved");
    let member = open
        && alert_incident_alerts::Entity::find()
            .select_only()
            .column(alert_incident_alerts::Column::AlertId)
            .filter(alert_incident_alerts::Column::IncidentId.eq(&row.incident_id))
            .filter(alert_incident_alerts::Column::AlertId.eq(&row.alert_id))
            .into_tuple::<String>()
            .one(&txn)
            .await?
            .is_some();
    if !member {
        txn.rollback().await?;
        return Ok(false);
    }
    upsert_member_with(&txn, row).await?;
    txn.commit().await?;
    Ok(true)
}

async fn upsert_member_with<C: ConnectionTrait>(
    conn: &C,
    row: incident_member_verdicts::Model,
) -> Result<(), errors::Error> {
    use incident_member_verdicts::Column as C;
    let active = incident_member_verdicts::ActiveModel {
        org_id: Set(row.org_id),
        incident_id: Set(row.incident_id),
        alert_id: Set(row.alert_id),
        verdict: Set(row.verdict),
        reason: Set(row.reason),
        evidence: Set(row.evidence),
        decided_by: Set(row.decided_by),
        signal: Set(row.signal),
        decided_by_user: Set(row.decided_by_user),
        model: Set(row.model),
        decided_at: Set(row.decided_at),
    };
    incident_member_verdicts::Entity::insert(active)
        .on_conflict(
            OnConflict::columns([C::IncidentId, C::AlertId])
                .update_columns([
                    C::Verdict,
                    C::Reason,
                    C::Evidence,
                    C::DecidedBy,
                    C::Signal,
                    C::DecidedByUser,
                    C::Model,
                    C::DecidedAt,
                ])
                .to_owned(),
        )
        .exec_without_returning(conn)
        .await?;
    Ok(())
}

pub async fn get_pair(
    org_id: &str,
    x: &str,
    y: &str,
) -> Result<Option<alert_pair_verdicts::Model>, errors::Error> {
    let (a, b) = pair_key(x, y);
    Ok(
        alert_pair_verdicts::Entity::find_by_id((org_id.to_string(), a, b))
            .one(get_orm_client_rw().await)
            .await?,
    )
}

/// Every unexpired remembered pair of the org, from the primary: a replica may lack a new choice.
pub async fn list_pairs(
    org_id: &str,
    now: i64,
) -> Result<Vec<alert_pair_verdicts::Model>, errors::Error> {
    use sea_orm::Condition;
    Ok(alert_pair_verdicts::Entity::find()
        .filter(alert_pair_verdicts::Column::OrgId.eq(org_id))
        .filter(
            Condition::any()
                .add(alert_pair_verdicts::Column::ExpiresAt.is_null())
                .add(alert_pair_verdicts::Column::ExpiresAt.gt(now)),
        )
        .all(get_orm_client_rw().await)
        .await?)
}

pub async fn put_pair(row: alert_pair_verdicts::Model) -> Result<(), errors::Error> {
    put_pair_with(get_orm_client_rw().await, row).await
}

pub async fn put_pair_with<C: ConnectionTrait>(
    conn: &C,
    row: alert_pair_verdicts::Model,
) -> Result<(), errors::Error> {
    use alert_pair_verdicts::Column as C;
    if row.alert_a > row.alert_b {
        return Err(Error::DbError(DbError::SeaORMError(
            "alert pair is not ordered".to_string(),
        )));
    }
    let active = alert_pair_verdicts::ActiveModel {
        org_id: Set(row.org_id),
        alert_a: Set(row.alert_a),
        alert_b: Set(row.alert_b),
        verdict: Set(row.verdict),
        reason: Set(row.reason),
        evidence: Set(row.evidence),
        decided_by: Set(row.decided_by),
        signal: Set(row.signal),
        decided_by_user: Set(row.decided_by_user),
        decided_at: Set(row.decided_at),
        expires_at: Set(row.expires_at),
    };
    alert_pair_verdicts::Entity::insert(active)
        .on_conflict(
            OnConflict::columns([C::OrgId, C::AlertA, C::AlertB])
                .update_columns([
                    C::Verdict,
                    C::Reason,
                    C::Evidence,
                    C::DecidedBy,
                    C::Signal,
                    C::DecidedByUser,
                    C::DecidedAt,
                    C::ExpiresAt,
                ])
                .to_owned(),
        )
        .exec_without_returning(conn)
        .await?;
    Ok(())
}

/// Whether `alert_id` was moved out of incidents this one's opener started (unexpired); read in
/// the join's transaction after it locked the incident row.
pub async fn separated_with<C: ConnectionTrait>(
    conn: &C,
    org_id: &str,
    incident_id: &str,
    alert_id: &str,
) -> Result<bool, errors::Error> {
    let Some(opener) = opener_with(conn, incident_id).await? else {
        return Ok(false);
    };
    if opener == alert_id {
        return Ok(false);
    }
    let (a, b) = pair_key(&opener, alert_id);
    let now = chrono::Utc::now().timestamp_micros();
    Ok(
        alert_pair_verdicts::Entity::find_by_id((org_id.to_string(), a, b))
            .one(conn)
            .await?
            .is_some_and(|pair| {
                pair.verdict == SEPARATE && pair.expires_at.is_none_or(|at| at > now)
            }),
    )
}

/// The incident's opener: the alert of its earliest firing.
pub async fn opener_of(incident_id: &str) -> Result<Option<String>, errors::Error> {
    opener_with(get_orm_client_rw().await, incident_id).await
}

async fn opener_with<C: ConnectionTrait>(
    conn: &C,
    incident_id: &str,
) -> Result<Option<String>, errors::Error> {
    Ok(alert_incident_alerts::Entity::find()
        .select_only()
        .column(alert_incident_alerts::Column::AlertId)
        .filter(alert_incident_alerts::Column::IncidentId.eq(incident_id))
        .order_by_asc(alert_incident_alerts::Column::AlertFiredAt)
        .order_by_asc(alert_incident_alerts::Column::AlertId)
        .into_tuple()
        .one(conn)
        .await?)
}

/// Fire times of the alert's firings in the org's incidents over `[since, until)`, oldest first.
pub async fn firing_times(
    org_id: &str,
    alert_id: &str,
    since: i64,
    until: i64,
) -> Result<Vec<i64>, errors::Error> {
    use sea_orm::{JoinType, RelationTrait};
    let times: Vec<i64> = alert_incident_alerts::Entity::find()
        .select_only()
        .column(alert_incident_alerts::Column::AlertFiredAt)
        .join(
            JoinType::InnerJoin,
            alert_incident_alerts::Relation::Incident.def(),
        )
        .filter(alert_incidents::Column::OrgId.eq(org_id))
        .filter(alert_incident_alerts::Column::AlertId.eq(alert_id))
        .filter(alert_incident_alerts::Column::AlertFiredAt.gte(since))
        .filter(alert_incident_alerts::Column::AlertFiredAt.lt(until))
        .order_by_asc(alert_incident_alerts::Column::AlertFiredAt)
        .into_tuple()
        .all(get_orm_client_ro().await)
        .await?;
    Ok(times)
}

/// The labels of the alert's latest firing in the incident that has them.
pub async fn latest_labels(
    incident_id: &str,
    alert_id: &str,
) -> Result<Option<String>, errors::Error> {
    let labels: Option<Option<String>> = alert_incident_alerts::Entity::find()
        .select_only()
        .column(alert_incident_alerts::Column::Labels)
        .filter(alert_incident_alerts::Column::IncidentId.eq(incident_id))
        .filter(alert_incident_alerts::Column::AlertId.eq(alert_id))
        .filter(alert_incident_alerts::Column::Labels.is_not_null())
        .order_by_desc(alert_incident_alerts::Column::AlertFiredAt)
        .into_tuple()
        .one(get_orm_client_rw().await)
        .await?;
    Ok(labels.flatten())
}

/// Earlier incidents holding both alerts, newest first, with their on-call causes.
pub async fn past_causes(
    org_id: &str,
    x: &str,
    y: &str,
    exclude: &str,
    limit: u64,
) -> Result<Vec<(String, Option<String>)>, errors::Error> {
    let db = get_orm_client_ro().await;
    let holding = |alert: &str| {
        alert_incident_alerts::Entity::find()
            .select_only()
            .column(alert_incident_alerts::Column::IncidentId)
            .filter(alert_incident_alerts::Column::AlertId.eq(alert.to_string()))
            .into_query()
    };
    use sea_orm::QueryTrait;
    let ids: Vec<String> = alert_incidents::Entity::find()
        .select_only()
        .column(alert_incidents::Column::Id)
        .filter(alert_incidents::Column::OrgId.eq(org_id))
        .filter(alert_incidents::Column::Id.ne(exclude))
        .filter(alert_incidents::Column::Id.in_subquery(holding(x)))
        .filter(alert_incidents::Column::Id.in_subquery(holding(y)))
        .order_by_desc(alert_incidents::Column::FirstAlertAt)
        .limit(limit)
        .into_tuple()
        .all(db)
        .await?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let causes: Vec<(Option<String>, Option<String>)> = oncall_responses::Entity::find()
        .select_only()
        .column(oncall_responses::Column::IncidentId)
        .column(oncall_responses::Column::Cause)
        .filter(oncall_responses::Column::OrgId.eq(org_id))
        .filter(oncall_responses::Column::IncidentId.is_in(ids.clone()))
        .filter(oncall_responses::Column::Cause.is_not_null())
        .into_tuple()
        .all(db)
        .await?;
    let by_incident: HashMap<String, String> = causes
        .into_iter()
        .filter_map(|(id, cause)| Some((id?, cause?)))
        .collect();
    Ok(ids
        .into_iter()
        .map(|id| {
            let cause = by_incident.get(&id).cloned();
            (id, cause)
        })
        .collect())
}

/// Deploys of the service (in the cluster when known) over `[since, until]`.
pub async fn deploys_between(
    org_id: &str,
    service: &str,
    cluster: Option<&str>,
    since: i64,
    until: i64,
) -> Result<Vec<service_version_deploys::Model>, errors::Error> {
    let mut query = service_version_deploys::Entity::find()
        .filter(service_version_deploys::Column::OrgId.eq(org_id))
        .filter(service_version_deploys::Column::ServiceName.eq(service))
        .filter(service_version_deploys::Column::At.gte(since))
        .filter(service_version_deploys::Column::At.lte(until));
    if let Some(cluster) = cluster {
        query = query.filter(service_version_deploys::Column::Cluster.eq(cluster));
    }
    Ok(query
        .order_by_desc(service_version_deploys::Column::At)
        .limit(3)
        .all(get_orm_client_ro().await)
        .await?)
}

pub async fn delete_by_org(org_id: &str) -> Result<(), errors::Error> {
    let db = get_orm_client_rw().await;
    incident_member_verdicts::Entity::delete_many()
        .filter(incident_member_verdicts::Column::OrgId.eq(org_id))
        .exec(db)
        .await?;
    alert_pair_verdicts::Entity::delete_many()
        .filter(alert_pair_verdicts::Column::OrgId.eq(org_id))
        .exec(db)
        .await?;
    Ok(())
}
