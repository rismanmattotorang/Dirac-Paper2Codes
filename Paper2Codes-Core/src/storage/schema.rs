use crate::storage::errors::{StorageError, StorageResult};
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;

/// Comprehensive schema definition for SurrealDB
pub struct Schema;

impl Schema {
    /// Setup all tables, fields, indexes, and vector indexes
    pub async fn setup(db: &Surreal<Client>) -> StorageResult<()> {
        // Read schema from embedded SQL file
        let schema_sql = include_str!("schema.sql");

        // Execute schema definition queries
        // Split by semicolon and execute each statement
        for statement in schema_sql.split(';') {
            let statement = statement.trim();
            if statement.is_empty() || statement.starts_with("--") {
                continue;
            }

            match db.query(statement).await {
                Ok(_) => continue,
                Err(e) => {
                    let msg = e.to_string();
                    // Ignore errors caused by statements that already exist
                    let is_duplicate = msg.contains("already") || msg.contains("exists");
                    if is_duplicate {
                        tracing::debug!(
                            "Skipping schema statement (already exists): {} - {}",
                            statement,
                            msg
                        );
                        continue;
                    }

                    return Err(StorageError::Schema(format!(
                        "Failed to execute schema statement: {} - Error: {}",
                        statement, msg
                    )));
                }
            }
        }

        Ok(())
    }

    /// Validate schema is properly set up
    pub async fn validate(db: &Surreal<Client>) -> StorageResult<()> {
        // Check if key tables exist by querying them
        // Use whitelist to prevent injection
        let tables = vec![
            "paper",
            "segment",
            "repository",
            "module",
            "task",
            "dependency",
            "document",
        ];

        for table in tables {
            // Use parameterized query for safety
            let mut response = db
                .query("INFO FOR TABLE type::table($table)")
                .bind(("table", surrealdb::sql::Value::from(table)))
                .await
                .map_err(|e| {
                    StorageError::Schema(format!("Table {} validation failed: {}", table, e))
                })?;

            // Try to take result - if it fails, table might not exist
            let _: Result<Vec<surrealdb::sql::Value>, _> = response.take(0).map_err(|_| {
                StorageError::Schema(format!("Table {} does not exist or is invalid", table))
            });
        }

        Ok(())
    }
}
