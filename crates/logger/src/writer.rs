//! LogWriter — enum-dispatched writers for different deployment contexts.

use std::sync::Arc;
use crate::types::LogEntry;

/// Pluggable log destination. Constructed once at startup.
pub enum LogWriter {
    /// Direct PG insert — for daemon (has DB access).
    #[cfg(feature = "pg")]
    Pg(sqlx_postgres::PgPool),

    /// HTTP POST to daemon /api/logs — for CLI, MCP, app.
    #[cfg(feature = "api")]
    Api { base_url: String, client: reqwest::Client },

    /// Buffer in memory until a PG pool is available (for bootstrap).
    /// Logs keep their original `logged_at`. On flush, `written_at` (default now())
    /// will differ — the gap shows buffering delay.
    Buffered(tokio::sync::Mutex<BufferedState>),

    /// No-op — for tests or when logging is disabled.
    Noop,
}

/// Internal state for the buffered writer.
pub struct BufferedState {
    pub(crate) buffer: Vec<LogEntry>,
    #[cfg(feature = "pg")]
    pub(crate) pool: Option<sqlx_postgres::PgPool>,
}

impl LogWriter {
    /// Create a PG writer from a connection pool.
    #[cfg(feature = "pg")]
    pub fn pg(pool: sqlx_postgres::PgPool) -> Arc<Self> {
        Arc::new(Self::Pg(pool))
    }

    /// Create an API writer targeting a daemon URL.
    #[cfg(feature = "api")]
    pub fn api(base_url: &str) -> Arc<Self> {
        Arc::new(Self::Api {
            base_url: base_url.to_string(),
            client: reqwest::Client::new(),
        })
    }

    /// Create a buffered writer that caches logs until `set_pool()` is called.
    pub fn buffered() -> Arc<Self> {
        Arc::new(Self::Buffered(tokio::sync::Mutex::new(BufferedState {
            buffer: Vec::new(),
            #[cfg(feature = "pg")]
            pool: None,
        })))
    }

    /// Provide a PG pool to a buffered writer, flushing all cached entries.
    #[cfg(feature = "pg")]
    pub async fn set_pool(&self, pool: sqlx_postgres::PgPool) {
        if let Self::Buffered(state) = self {
            let mut guard = state.lock().await;
            for entry in guard.buffer.drain(..) {
                let _ = Self::write_to_pg(&pool, &entry).await;
            }
            guard.pool = Some(pool);
        }
    }

    /// Create a noop writer.
    pub fn noop() -> Arc<Self> {
        Arc::new(Self::Noop)
    }

    #[cfg(feature = "pg")]
    pub(crate) async fn write_to_pg(pool: &sqlx_postgres::PgPool, entry: &LogEntry) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO public.logs(level, running_on, logged_at, message, context, data, error)
             VALUES($1, $2, $3::timestamptz, $4, $5, $6, $7)"
        )
        .bind(&entry.level)
        .bind(&entry.running_on)
        .bind(&entry.logged_at)
        .bind(&entry.message)
        .bind(&entry.context)
        .bind(&entry.data)
        .bind(&entry.error)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub(crate) async fn write(&self, entry: &LogEntry) {
        match self {
            #[cfg(feature = "pg")]
            Self::Pg(pool) => {
                let _ = Self::write_to_pg(pool, entry).await;
            }
            #[cfg(feature = "api")]
            Self::Api { base_url, client } => {
                let _ = client
                    .post(format!("{base_url}/api/logs"))
                    .json(entry)
                    .send()
                    .await;
            }
            Self::Buffered(state) => {
                let mut guard = state.lock().await;
                #[cfg(feature = "pg")]
                if let Some(pool) = &guard.pool {
                    let _ = Self::write_to_pg(pool, entry).await;
                    return;
                }
                guard.buffer.push(entry.clone());
            }
            Self::Noop => {}
        }
    }
}
