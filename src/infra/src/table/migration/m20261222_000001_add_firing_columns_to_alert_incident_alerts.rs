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

//! The firing's own labels and severity on each incident membership row.

use sea_orm_migration::prelude::*;

const TABLE: &str = "alert_incident_alerts";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite drops IF NOT EXISTS, so a retry must skip a column already added.
        for (name, column) in columns() {
            if !manager.has_column(TABLE, name).await? {
                manager.alter_table(add_column_statement(column)).await?;
            }
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for (name, _) in columns() {
            if manager.has_column(TABLE, name).await? {
                let drop = Table::alter()
                    .table(AlertIncidentAlerts::Table)
                    .drop_column(Alias::new(name))
                    .to_owned();
                manager.alter_table(drop).await?;
            }
        }
        Ok(())
    }
}

#[derive(DeriveIden)]
enum AlertIncidentAlerts {
    Table,
    Labels,
    Severity,
}

fn columns() -> [(&'static str, ColumnDef); 2] {
    [
        (
            "labels",
            ColumnDef::new(AlertIncidentAlerts::Labels)
                .text()
                .null()
                .to_owned(),
        ),
        (
            "severity",
            ColumnDef::new(AlertIncidentAlerts::Severity)
                .string_len(8)
                .null()
                .to_owned(),
        ),
    ]
}

fn add_column_statement(mut column: ColumnDef) -> TableAlterStatement {
    Table::alter()
        .table(AlertIncidentAlerts::Table)
        .add_column(&mut column)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use sea_orm::{ConnectionTrait, Database};

    use super::*;

    #[tokio::test]
    async fn existing_rows_have_no_labels_or_severity_and_up_down_up_round_trips() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.execute_unprepared(
            "CREATE TABLE alert_incident_alerts (incident_id TEXT, alert_id TEXT, alert_fired_at BIGINT)",
        )
        .await
        .unwrap();
        db.execute_unprepared("INSERT INTO alert_incident_alerts VALUES ('i', 'a', 1)")
            .await
            .unwrap();
        let manager = SchemaManager::new(&db);
        Migration.up(&manager).await.unwrap();
        Migration.up(&manager).await.unwrap();
        let sql = "SELECT labels, severity FROM alert_incident_alerts WHERE incident_id = 'i'";
        let stmt = sea_orm::Statement::from_string(db.get_database_backend(), sql);
        let row = db.query_one(stmt).await.unwrap().unwrap();
        assert_eq!(row.try_get::<Option<String>>("", "labels").unwrap(), None);
        assert_eq!(row.try_get::<Option<String>>("", "severity").unwrap(), None);
        Migration.down(&manager).await.unwrap();
        for name in ["labels", "severity"] {
            assert!(!manager.has_column(TABLE, name).await.unwrap());
        }
        Migration.up(&manager).await.unwrap();
        for name in ["labels", "severity"] {
            assert!(manager.has_column(TABLE, name).await.unwrap());
        }
    }
}
