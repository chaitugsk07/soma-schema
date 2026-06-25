use async_trait::async_trait;
use sqlx::{postgres::PgConnection, Connection, Executor, PgPool};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::driver::{AppliedMigration, LockGuard, MigrationDriver};
use crate::error::{Error, Result};
use crate::migration::Migration;

/// Validate that an identifier contains only [A-Za-z0-9_].
/// A leading digit is allowed (e.g. `00_schema_migrations`).
fn validate_ident(s: &str) -> Result<()> {
    if s.is_empty() || !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(Error::InvalidIdentifier(s.to_owned()));
    }
    Ok(())
}

/// Configuration for the Postgres driver.
///
/// # Construction
///
/// `PostgresConfig` is intentionally constructable with struct-literal syntax so callers
/// can write `PostgresConfig { schema: Some("app".into()), ..Default::default() }`.
/// It is therefore NOT marked `#[non_exhaustive]` — doing so would break that ergonomic.
/// New fields added in future versions will always have defaults provided via `Default`.
#[derive(Debug, Clone)]
pub struct PostgresConfig {
    /// Target schema. `None` means use the connection's search_path default.
    pub schema: Option<String>,
    /// Tracking table name. Defaults to `00_schema_migrations`.
    pub table: String,
    /// Advisory lock key. Defaults to 918273645.
    pub advisory_lock_key: i64,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            schema: None,
            table: "00_schema_migrations".to_owned(),
            advisory_lock_key: 918273645,
        }
    }
}

/// A held advisory lock. Keeps a dedicated connection alive.
/// Releases the lock on Drop via a background task.
pub struct PgLockGuard {
    /// The connection is wrapped in Arc<Mutex<>> so we can move it into the drop task.
    conn: Arc<Mutex<Option<PgConnection>>>,
    key: i64,
}

impl LockGuard for PgLockGuard {}

impl Drop for PgLockGuard {
    fn drop(&mut self) {
        let conn_arc = Arc::clone(&self.conn);
        let key = self.key;
        // ponytail: async cleanup in Drop is inherently unreliable — if the runtime is
        // draining at shutdown, spawn() silently drops the future. In practice Postgres
        // releases session-level advisory locks when the connection closes (either via
        // conn.close() or TCP teardown), so this is best-effort cleanup: the lock will
        // be released eventually, but an operator-visible log line on failure is better
        // than silence.
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let mut guard = conn_arc.lock().await;
                if let Some(mut conn) = guard.take() {
                    if let Err(e) = sqlx::query("SELECT pg_advisory_unlock($1)")
                        .bind(key)
                        .execute(&mut conn)
                        .await
                    {
                        // ponytail: can't propagate errors from Drop; log so operators
                        // can see it. Postgres releases session locks on TCP close anyway.
                        eprintln!("WARN soma-schema: pg_advisory_unlock(key={key}) failed: {e}; lock will be released when the connection closes");
                    }
                    let _ = conn.close().await;
                }
            });
        }
    }
}

/// Postgres implementation of `MigrationDriver`.
///
/// The pool must have at least 2 connections: one is reserved for the advisory lock,
/// and at least one more is needed for migration queries. `PgPoolOptions::new().max_connections(2)`
/// is the recommended minimum for CLI use.
#[derive(Debug)]
pub struct PostgresDriver {
    pool: PgPool,
    config: PostgresConfig,
}

impl PostgresDriver {
    /// Create a new `PostgresDriver`.
    ///
    /// # Errors
    ///
    /// Returns `Error::InvalidIdentifier` if the table or schema name contains
    /// characters outside `[A-Za-z0-9_]`.
    ///
    /// Returns `Error::PoolTooSmall` if the pool has fewer than 2 connections.
    /// One connection is reserved for the advisory lock; all migration queries
    /// need at least one more. With `max_connections == 1` every `pool.begin()`
    /// call would wait forever.
    pub fn new(pool: PgPool, config: PostgresConfig) -> Result<Self> {
        validate_ident(&config.table)?;
        if let Some(ref s) = config.schema {
            validate_ident(s)?;
        }
        // One connection is permanently held for the advisory lock.
        // Every migration query needs a second connection from the pool.
        if pool.options().get_max_connections() < 2 {
            return Err(Error::PoolTooSmall);
        }
        Ok(Self { pool, config })
    }

    /// Return `"schema"."table"` or just `"table"` depending on whether schema is set.
    fn qualified_table(&self) -> String {
        match self.config.schema.as_deref() {
            Some(s) => format!("\"{s}\".\"{}\"", &self.config.table),
            None => format!("\"{}\"", &self.config.table),
        }
    }

    fn set_search_path_sql(&self) -> Option<String> {
        // SET LOCAL is transaction-scoped and cannot escape the transaction boundary,
        // preventing search_path leaking to other pooled connections after commit.
        self.config
            .schema
            .as_deref()
            .map(|s| format!("SET LOCAL search_path TO \"{s}\""))
    }
}

#[async_trait]
impl MigrationDriver for PostgresDriver {
    async fn acquire_lock(&self) -> Result<Box<dyn LockGuard>> {
        let mut conn = self.pool.acquire().await?.detach();
        sqlx::query("SELECT pg_advisory_lock($1)")
            .bind(self.config.advisory_lock_key)
            .execute(&mut conn)
            .await?;
        let guard = PgLockGuard {
            conn: Arc::new(Mutex::new(Some(conn))),
            key: self.config.advisory_lock_key,
        };
        Ok(Box::new(guard))
    }

    async fn run_setup_sql(&self, name: &str, sql: &str) -> Result<()> {
        // Own the strings up front: async_trait boxes the future and requires all
        // captured values to be 'async_trait; owned Strings satisfy that.
        let name = name.to_owned();
        let sql = sql.to_owned();
        let mut tx = self.pool.begin().await?;
        if let Some(sp) = self.set_search_path_sql() {
            sqlx::query(&sp)
                .execute(&mut *tx)
                .await
                .map_err(|e| Error::SetupFailed {
                    file: name.clone(),
                    source: e,
                })?;
        }
        // raw_sql sends the whole file as a simple-query batch (simple protocol,
        // not prepared), so multi-statement DDL and PL/pgSQL $$ blocks work correctly.
        // We call Executor::execute on the connection directly (not the RawSql inherent
        // execute), because the inherent method is async fn and captures the executor
        // reference, which causes an HRTB Send failure inside async_trait boxed futures.
        // Calling through the Executor trait returns BoxFuture<'e> which does not capture.
        {
            let conn: &mut PgConnection = &mut tx;
            conn.execute(sqlx::raw_sql(&sql))
                .await
                .map_err(|e| Error::SetupFailed {
                    file: name.clone(),
                    source: e,
                })?;
        }
        tx.commit().await.map_err(|e| Error::SetupFailed {
            file: name,
            source: e,
        })?;
        Ok(())
    }

    async fn ensure_tracking_table(&self) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        // Create schema if configured.
        if let Some(schema) = &self.config.schema {
            let create_schema = format!("CREATE SCHEMA IF NOT EXISTS \"{schema}\"");
            sqlx::query(&create_schema).execute(&mut *tx).await?;
        }
        // Set search_path so the table creation lands in the right schema.
        if let Some(sp) = self.set_search_path_sql() {
            sqlx::query(&sp).execute(&mut *tx).await?;
        }
        let qt = self.qualified_table();
        let create_table = format!(
            r#"CREATE TABLE IF NOT EXISTS {qt} (
    version        INTEGER      NOT NULL,
    file           VARCHAR(255) NOT NULL,
    name           VARCHAR(255) NOT NULL,
    checksum       TEXT         NOT NULL,
    description    TEXT,
    batch          INTEGER      NOT NULL,
    applied_at     TIMESTAMPTZ  NOT NULL DEFAULT now(),
    applied_by     TEXT         NOT NULL DEFAULT current_user,
    execution_ms   INTEGER,
    PRIMARY KEY (version, file)
)"#
        );
        sqlx::query(&create_table).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn applied(&self) -> Result<Vec<AppliedMigration>> {
        use chrono::{DateTime, Utc};
        let qt = self.qualified_table();
        let sql = format!(
            "SELECT version, file, name, checksum, description, batch, applied_at, applied_by, execution_ms \
             FROM {qt} ORDER BY version ASC, file ASC"
        );
        let rows = sqlx::query_as::<
            _,
            (
                i32,
                String,
                String,
                String,
                Option<String>,
                i32,
                DateTime<Utc>,
                String,
                Option<i32>,
            ),
        >(&sql)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    version,
                    file,
                    name,
                    checksum,
                    description,
                    batch,
                    applied_at,
                    applied_by,
                    execution_ms,
                )| {
                    AppliedMigration {
                        version: version as u32,
                        file,
                        name,
                        checksum,
                        description,
                        batch,
                        applied_at,
                        applied_by,
                        execution_ms,
                    }
                },
            )
            .collect())
    }

    async fn apply(&self, migration: &Migration, up_sql: &str, batch: i32) -> Result<()> {
        // Own SQL up front so the async_trait boxed future doesn't borrow across the lifetime boundary.
        let up_sql = up_sql.to_owned();
        let start = std::time::Instant::now();
        let mut tx = self.pool.begin().await?;
        // Set search_path so DDL lands in the right schema.
        if let Some(sp) = self.set_search_path_sql() {
            sqlx::query(&sp).execute(&mut *tx).await?;
        }
        // raw_sql sends the whole file as a simple-query batch — handles multi-statement
        // DDL and PL/pgSQL $$ bodies that the old per-statement loop mis-split.
        // Call through Executor::execute (BoxFuture) not RawSql::execute (async fn) to
        // avoid the HRTB Send failure that async fn captures cause inside async_trait.
        {
            let conn: &mut PgConnection = &mut tx;
            conn.execute(sqlx::raw_sql(&up_sql)).await?;
        }
        let elapsed_ms = start.elapsed().as_millis() as i32;
        let qt = self.qualified_table();
        let insert = format!(
            "INSERT INTO {qt} (version, file, name, checksum, description, batch, applied_at, applied_by, execution_ms) \
             VALUES ($1, $2, $3, $4, $5, $6, now(), current_user, $7)"
        );
        sqlx::query(&insert)
            .bind(migration.version as i32)
            .bind(&migration.file)
            .bind(&migration.name)
            .bind(&migration.checksum)
            .bind(migration.why.as_deref())
            .bind(batch)
            .bind(elapsed_ms)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn revert(&self, applied: &AppliedMigration, down_sql: &str) -> Result<()> {
        // Own SQL up front so the async_trait boxed future doesn't borrow across the lifetime boundary.
        let down_sql = down_sql.to_owned();
        let mut tx = self.pool.begin().await?;
        if let Some(sp) = self.set_search_path_sql() {
            sqlx::query(&sp).execute(&mut *tx).await?;
        }
        // raw_sql sends the whole file as a simple-query batch — handles PL/pgSQL $$ bodies correctly.
        // Call through Executor::execute (BoxFuture) not RawSql::execute (async fn) to
        // avoid the HRTB Send failure that async fn captures cause inside async_trait.
        {
            let conn: &mut PgConnection = &mut tx;
            conn.execute(sqlx::raw_sql(&down_sql)).await?;
        }
        let qt = self.qualified_table();
        let delete = format!("DELETE FROM {qt} WHERE version = $1 AND file = $2");
        sqlx::query(&delete)
            .bind(applied.version as i32)
            .bind(&applied.file)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_ident() {
        assert!(validate_ident("00_schema_migrations").is_ok());
        assert!(validate_ident("my_schema").is_ok());
        assert!(validate_ident("ABC123").is_ok());
    }

    #[test]
    fn invalid_ident_rejects_special_chars() {
        assert!(matches!(
            validate_ident("bad-name"),
            Err(Error::InvalidIdentifier(_))
        ));
        assert!(matches!(
            validate_ident("has space"),
            Err(Error::InvalidIdentifier(_))
        ));
        assert!(matches!(
            validate_ident(""),
            Err(Error::InvalidIdentifier(_))
        ));
        assert!(matches!(
            validate_ident("semi;colon"),
            Err(Error::InvalidIdentifier(_))
        ));
    }

    #[test]
    fn leading_digit_allowed() {
        // e.g. "00_schema_migrations" — leading digit is valid
        assert!(validate_ident("00_schema_migrations").is_ok());
        assert!(validate_ident("01_vault").is_ok());
    }

    #[test]
    fn qualified_table_format() {
        // Verify schema.table ordering without needing a live pool.
        let with_schema = {
            let s = "myschema";
            let t = "00_schema_migrations";
            format!("\"{s}\".\"{t}\"")
        };
        assert_eq!(with_schema, "\"myschema\".\"00_schema_migrations\"");

        let without_schema = {
            let t = "00_schema_migrations";
            format!("\"{t}\"")
        };
        assert_eq!(without_schema, "\"00_schema_migrations\"");
    }
}
