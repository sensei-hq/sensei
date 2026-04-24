use sqlx_postgres::PgPool;

/// PostgreSQL store — replaces SQLite Store during migration.
/// Schema is managed by `dbd apply`, not by this code.
/// Callers will be wired in as entities migrate (issues #101–#111).
#[allow(dead_code)]
pub struct PgStore {
    pool: PgPool,
}

#[allow(dead_code)]
impl PgStore {
    /// Connect to a PostgreSQL database.
    pub async fn connect(database_url: &str) -> Result<Self, String> {
        let pool = PgPool::connect(database_url)
            .await
            .map_err(|e| format!("PgStore connect: {}", e))?;
        Ok(Self { pool })
    }

    /// Get a reference to the connection pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // ── Config ────────────────────────────────────────────────────────

    pub async fn get_config(&self, key: &str) -> Result<Option<String>, String> {
        let row: Option<(String,)> = sqlx_core::query_as::query_as(
            "SELECT value FROM sensei.config WHERE key = $1"
        )
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(row.map(|r| r.0))
    }

    pub async fn set_config(&self, key: &str, value: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.config(key, value) VALUES($1, $2) ON CONFLICT(key) DO UPDATE SET value = EXCLUDED.value"
        )
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn delete_config(&self, key: &str) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.config WHERE key = $1")
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_all_config(&self) -> Result<std::collections::HashMap<String, String>, String> {
        let rows: Vec<(String, String)> = sqlx_core::query_as::query_as(
            "SELECT key, value FROM sensei.config"
        )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().collect())
    }

    // ── Tags (controlled vocabulary) ──────────────────────────────────

    pub async fn add_tag(&self, tag: &str, category: Option<&str>) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.tags(tag, category) VALUES($1, $2) ON CONFLICT(tag) DO UPDATE SET category = EXCLUDED.category, modified_at = now()"
        )
            .bind(tag)
            .bind(category)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn remove_tag(&self, tag: &str) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.tags WHERE tag = $1")
            .bind(tag)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn list_tags(&self) -> Result<Vec<(String, Option<String>)>, String> {
        sqlx_core::query_as::query_as("SELECT tag, category FROM sensei.tags ORDER BY tag")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn list_tags_by_category(&self, category: &str) -> Result<Vec<String>, String> {
        let rows: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT tag FROM sensei.tags WHERE category = $1 ORDER BY tag"
        )
            .bind(category)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    // ── Raw ──────────────────────────────────────────────────────────

    /// Execute a raw SQL statement (for one-off queries like scanned_roots).
    pub async fn execute_raw(&self, sql: &str) -> Result<(), String> {
        sqlx_core::query::query(sql)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("PgStore execute_raw: {}", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx_core::query_as::query_as;

    fn test_db_url() -> String {
        std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost:5432/sensei".to_string())
    }

    #[tokio::test]
    async fn connect_to_pg() {
        let store = PgStore::connect(&test_db_url()).await.unwrap();
        let row: (i32,) = query_as("SELECT 1")
            .fetch_one(store.pool())
            .await
            .unwrap();
        assert_eq!(row.0, 1);
    }

    #[tokio::test]
    async fn execute_raw_works() {
        let store = PgStore::connect(&test_db_url()).await.unwrap();
        store.execute_raw("SELECT 1").await.unwrap();
    }

    #[tokio::test]
    async fn schema_exists() {
        let store = PgStore::connect(&test_db_url()).await.unwrap();
        let row: (bool,) = query_as(
            "SELECT EXISTS(SELECT 1 FROM information_schema.schemata WHERE schema_name = 'sensei')"
        )
            .fetch_one(store.pool())
            .await
            .unwrap();
        assert!(row.0, "sensei schema must exist — run `dbd apply` first");
    }

    // ── Config tests ───────────────────────────────────────────────

    async fn pg_store() -> PgStore {
        PgStore::connect(&test_db_url()).await.unwrap()
    }

    /// Generate a unique key prefix for test isolation.
    fn tkey(test: &str, key: &str) -> String {
        format!("_test:{}:{}", test, key)
    }

    #[tokio::test]
    async fn config_set_and_get() {
        let s = pg_store().await;
        let k = tkey("set_get", "theme");
        s.set_config(&k, "dark").await.unwrap();
        assert_eq!(s.get_config(&k).await.unwrap(), Some("dark".into()));
        s.delete_config(&k).await.unwrap(); // cleanup
    }

    #[tokio::test]
    async fn config_get_missing_returns_none() {
        let s = pg_store().await;
        assert_eq!(s.get_config("_test:missing:nonexistent").await.unwrap(), None);
    }

    #[tokio::test]
    async fn config_set_overwrites() {
        let s = pg_store().await;
        let k = tkey("overwrite", "k");
        s.set_config(&k, "v1").await.unwrap();
        s.set_config(&k, "v2").await.unwrap();
        assert_eq!(s.get_config(&k).await.unwrap(), Some("v2".into()));
        s.delete_config(&k).await.unwrap();
    }

    #[tokio::test]
    async fn config_delete() {
        let s = pg_store().await;
        let k = tkey("delete", "k");
        s.set_config(&k, "v").await.unwrap();
        s.delete_config(&k).await.unwrap();
        assert_eq!(s.get_config(&k).await.unwrap(), None);
    }

    #[tokio::test]
    async fn config_delete_nonexistent_is_noop() {
        let s = pg_store().await;
        s.delete_config("_test:noop:nope").await.unwrap();
    }

    #[tokio::test]
    async fn config_get_all() {
        let s = pg_store().await;
        let k1 = tkey("getall", "a");
        let k2 = tkey("getall", "b");
        s.set_config(&k1, "1").await.unwrap();
        s.set_config(&k2, "2").await.unwrap();
        let all = s.get_all_config().await.unwrap();
        assert_eq!(all[&k1], "1");
        assert_eq!(all[&k2], "2");
        s.delete_config(&k1).await.unwrap();
        s.delete_config(&k2).await.unwrap();
    }

    // ── Tags tests ────────────────────────────────────────────────────

    #[tokio::test]
    async fn tag_add_and_list() {
        let s = pg_store().await;
        let tag = "_test:tag_add:rust";
        s.add_tag(tag, Some("stack")).await.unwrap();
        let tags = s.list_tags().await.unwrap();
        assert!(tags.iter().any(|(t, c)| t == tag && c.as_deref() == Some("stack")));
        s.remove_tag(tag).await.unwrap();
    }

    #[tokio::test]
    async fn tag_add_without_category() {
        let s = pg_store().await;
        let tag = "_test:tag_nocat:misc";
        s.add_tag(tag, None).await.unwrap();
        let tags = s.list_tags().await.unwrap();
        assert!(tags.iter().any(|(t, c)| t == tag && c.is_none()));
        s.remove_tag(tag).await.unwrap();
    }

    #[tokio::test]
    async fn tag_add_duplicate_is_upsert() {
        let s = pg_store().await;
        let tag = "_test:tag_dup:ts";
        s.add_tag(tag, Some("stack")).await.unwrap();
        s.add_tag(tag, Some("language")).await.unwrap(); // update category
        let tags = s.list_tags().await.unwrap();
        let found: Vec<_> = tags.iter().filter(|(t, _)| t == tag).collect();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1.as_deref(), Some("language"));
        s.remove_tag(tag).await.unwrap();
    }

    #[tokio::test]
    async fn tag_remove() {
        let s = pg_store().await;
        let tag = "_test:tag_rm:go";
        s.add_tag(tag, Some("stack")).await.unwrap();
        s.remove_tag(tag).await.unwrap();
        let tags = s.list_tags().await.unwrap();
        assert!(!tags.iter().any(|(t, _)| t == tag));
    }

    #[tokio::test]
    async fn tag_remove_nonexistent_is_noop() {
        let s = pg_store().await;
        s.remove_tag("_test:tag_rm_noop:xyz").await.unwrap();
    }

    #[tokio::test]
    async fn tag_list_by_category() {
        let s = pg_store().await;
        let t1 = "_test:tag_cat:rust";
        let t2 = "_test:tag_cat:ts";
        let t3 = "_test:tag_cat:active";
        s.add_tag(t1, Some("stack")).await.unwrap();
        s.add_tag(t2, Some("stack")).await.unwrap();
        s.add_tag(t3, Some("status")).await.unwrap();
        let stack_tags = s.list_tags_by_category("stack").await.unwrap();
        assert!(stack_tags.contains(&t1.to_string()));
        assert!(stack_tags.contains(&t2.to_string()));
        assert!(!stack_tags.contains(&t3.to_string()));
        s.remove_tag(t1).await.unwrap();
        s.remove_tag(t2).await.unwrap();
        s.remove_tag(t3).await.unwrap();
    }

    // ── Schema tests ─────────────────────────────────────────────────

    #[tokio::test]
    async fn memories_table_exists() {
        let store = PgStore::connect(&test_db_url()).await.unwrap();
        let row: (bool,) = query_as(
            "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema = 'sensei' AND table_name = 'memories')"
        )
            .fetch_one(store.pool())
            .await
            .unwrap();
        assert!(row.0, "sensei.memories table must exist — run `dbd apply` first");
    }
}
