//! Call sites with readable arguments (spec 20 §7b, #251): the SQL. Which fact
//! becomes which row, and how receivers and arguments are spelled, is
//! `crate::indexer::persist`'s.

use super::*;

/// One call site as `persist` hands it over: already spelled for storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallSiteColumns {
    pub from_node_id: uuid::Uuid,
    pub at: serde_json::Value,
    pub callee: String,
    pub type_args: Vec<String>,
    pub receiver: serde_json::Value,
    pub args: serde_json::Value,
}

/// One stored call site, raw — `persist` decodes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredCallSite {
    pub file_path: String,
    pub from_fqn: String,
    pub at: serde_json::Value,
    pub callee: String,
    pub type_args: Vec<String>,
    pub receiver: serde_json::Value,
    pub args: serde_json::Value,
}

impl PgStore {
    /// Replace the call sites `file_path` states, in one transaction. An empty
    /// `calls` clears the file — a re-index that found none must not leave the
    /// last pass's behind.
    pub async fn replace_call_sites(
        &self,
        folder_id: &uuid::Uuid,
        file_path: &str,
        calls: &[CallSiteColumns],
    ) -> Result<(), String> {
        let mut tx = self.pool.begin().await.map_err(|e| format!("call sites: begin: {e}"))?;
        sqlx_core::query::query(
            "DELETE FROM sensei.call_sites WHERE folder_id = $1 AND file_path = $2",
        )
        .bind(folder_id)
        .bind(file_path)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("call sites: clear {file_path}: {e}"))?;
        for c in calls {
            sqlx_core::query::query(
                "INSERT INTO sensei.call_sites
                        (folder_id, file_path, from_node_id, at, callee, type_args, receiver, args)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(folder_id)
            .bind(file_path)
            .bind(c.from_node_id)
            .bind(&c.at)
            .bind(&c.callee)
            .bind(&c.type_args)
            .bind(&c.receiver)
            .bind(&c.args)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("call sites: {file_path} {}: {e}", c.callee))?;
        }
        tx.commit().await.map_err(|e| format!("call sites: commit: {e}"))
    }

    /// Every call site stored for `folder_id`, in write order.
    pub async fn call_sites_of(
        &self,
        folder_id: &uuid::Uuid,
    ) -> Result<Vec<StoredCallSite>, String> {
        type Row = (
            String,
            String,
            serde_json::Value,
            String,
            Vec<String>,
            serde_json::Value,
            serde_json::Value,
        );
        let rows: Vec<Row> = sqlx_core::query_as::query_as(
            "SELECT c.file_path, n.fqn, c.at, c.callee, c.type_args, c.receiver, c.args
               FROM sensei.call_sites c
               JOIN sensei.nodes n ON n.id = c.from_node_id
              WHERE c.folder_id = $1
              ORDER BY c.id",
        )
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("call sites: read: {e}"))?;
        Ok(rows
            .into_iter()
            .map(|(file_path, from_fqn, at, callee, type_args, receiver, args)| StoredCallSite {
                file_path,
                from_fqn,
                at,
                callee,
                type_args,
                receiver,
                args,
            })
            .collect())
    }
}
