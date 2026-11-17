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
//! Membership verdicts per incident member, and the per-org memory of alert pairs.

use sea_orm_migration::prelude::*;

const MEMBER_ORG_INDEX: &str = "idx_incident_member_verdicts_org_incident";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(member_verdicts_statement()).await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name(MEMBER_ORG_INDEX)
                    .table(IncidentMemberVerdicts::Table)
                    .col(IncidentMemberVerdicts::OrgId)
                    .col(IncidentMemberVerdicts::IncidentId)
                    .to_owned(),
            )
            .await?;
        manager.create_table(pair_verdicts_statement()).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(AlertPairVerdicts::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(IncidentMemberVerdicts::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum IncidentMemberVerdicts {
    Table,
    OrgId,
    IncidentId,
    AlertId,
    Verdict,
    Reason,
    Evidence,
    DecidedBy,
    Signal,
    DecidedByUser,
    Model,
    DecidedAt,
}

#[derive(DeriveIden)]
enum AlertPairVerdicts {
    Table,
    OrgId,
    AlertA,
    AlertB,
    Verdict,
    Reason,
    Evidence,
    DecidedBy,
    Signal,
    DecidedByUser,
    DecidedAt,
    ExpiresAt,
}

fn member_verdicts_statement() -> TableCreateStatement {
    Table::create()
        .table(IncidentMemberVerdicts::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(IncidentMemberVerdicts::OrgId)
                .string_len(256)
                .not_null(),
        )
        .col(
            ColumnDef::new(IncidentMemberVerdicts::IncidentId)
                .string_len(256)
                .not_null(),
        )
        .col(
            ColumnDef::new(IncidentMemberVerdicts::AlertId)
                .string_len(256)
                .not_null(),
        )
        .col(
            ColumnDef::new(IncidentMemberVerdicts::Verdict)
                .string_len(16)
                .not_null(),
        )
        .col(
            ColumnDef::new(IncidentMemberVerdicts::Reason)
                .text()
                .not_null(),
        )
        .col(
            ColumnDef::new(IncidentMemberVerdicts::Evidence)
                .text()
                .not_null(),
        )
        .col(
            ColumnDef::new(IncidentMemberVerdicts::DecidedBy)
                .string_len(8)
                .not_null(),
        )
        .col(
            ColumnDef::new(IncidentMemberVerdicts::Signal)
                .string_len(32)
                .null(),
        )
        .col(
            ColumnDef::new(IncidentMemberVerdicts::DecidedByUser)
                .string_len(256)
                .null(),
        )
        .col(
            ColumnDef::new(IncidentMemberVerdicts::Model)
                .string_len(128)
                .null(),
        )
        .col(
            ColumnDef::new(IncidentMemberVerdicts::DecidedAt)
                .big_integer()
                .not_null(),
        )
        .primary_key(
            Index::create()
                .col(IncidentMemberVerdicts::IncidentId)
                .col(IncidentMemberVerdicts::AlertId),
        )
        .to_owned()
}

fn pair_verdicts_statement() -> TableCreateStatement {
    Table::create()
        .table(AlertPairVerdicts::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(AlertPairVerdicts::OrgId)
                .string_len(256)
                .not_null(),
        )
        .col(
            ColumnDef::new(AlertPairVerdicts::AlertA)
                .string_len(256)
                .not_null(),
        )
        .col(
            ColumnDef::new(AlertPairVerdicts::AlertB)
                .string_len(256)
                .not_null(),
        )
        .col(
            ColumnDef::new(AlertPairVerdicts::Verdict)
                .string_len(16)
                .not_null(),
        )
        .col(ColumnDef::new(AlertPairVerdicts::Reason).text().not_null())
        .col(
            ColumnDef::new(AlertPairVerdicts::Evidence)
                .text()
                .not_null(),
        )
        .col(
            ColumnDef::new(AlertPairVerdicts::DecidedBy)
                .string_len(8)
                .not_null(),
        )
        .col(
            ColumnDef::new(AlertPairVerdicts::Signal)
                .string_len(32)
                .null(),
        )
        .col(
            ColumnDef::new(AlertPairVerdicts::DecidedByUser)
                .string_len(256)
                .null(),
        )
        .col(
            ColumnDef::new(AlertPairVerdicts::DecidedAt)
                .big_integer()
                .not_null(),
        )
        .col(
            ColumnDef::new(AlertPairVerdicts::ExpiresAt)
                .big_integer()
                .null(),
        )
        .primary_key(
            Index::create()
                .col(AlertPairVerdicts::OrgId)
                .col(AlertPairVerdicts::AlertA)
                .col(AlertPairVerdicts::AlertB),
        )
        .to_owned()
}

#[cfg(test)]
mod tests {
    use sea_orm::{ConnectionTrait, Database};

    use super::*;

    #[tokio::test]
    async fn tables_round_trip_up_down_up() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let manager = SchemaManager::new(&db);
        Migration.up(&manager).await.unwrap();
        Migration.up(&manager).await.unwrap();
        db.execute_unprepared(
            "INSERT INTO incident_member_verdicts VALUES \
             ('o', 'i', 'a', 'unrelated', 'r', '{}', 'rule', 'workload', NULL, NULL, 1)",
        )
        .await
        .unwrap();
        db.execute_unprepared(
            "INSERT INTO alert_pair_verdicts VALUES \
             ('o', 'a', 'b', 'separate', 'r', '{}', 'user', NULL, 'u', 1, NULL)",
        )
        .await
        .unwrap();
        Migration.down(&manager).await.unwrap();
        for table in ["incident_member_verdicts", "alert_pair_verdicts"] {
            assert!(!manager.has_table(table).await.unwrap(), "{table} dropped");
        }
        Migration.up(&manager).await.unwrap();
        assert!(
            manager
                .has_index("incident_member_verdicts", MEMBER_ORG_INDEX)
                .await
                .unwrap()
        );
    }
}
