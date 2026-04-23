use rusqlite::{params, OptionalExtension};
use super::super::Store;

impl Store {
    pub fn get_config(&self, key: &str) -> rusqlite::Result<Option<String>> {
        self.conn.query_row(
            "SELECT value FROM config WHERE key = ?1",
            params![key],
            |row| row.get(0),
        ).optional()
    }

    pub fn set_config(&self, key: &str, value: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO config(key, value) VALUES(?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn delete_config(&self, key: &str) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM config WHERE key = ?1", params![key])?;
        Ok(())
    }

    pub fn get_all_config(&self) -> rusqlite::Result<std::collections::HashMap<String, String>> {
        let mut stmt = self.conn.prepare("SELECT key, value FROM config")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut map = std::collections::HashMap::new();
        for (k, v) in rows.flatten() { map.insert(k, v); }
        Ok(map)
    }

    pub fn execute_raw(&self, sql: &str) -> rusqlite::Result<()> {
        self.conn.execute_batch(sql)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> Store { Store::open_memory().unwrap() }

    #[test]
    fn set_and_get() {
        let s = test_store();
        s.set_config("theme", "dark").unwrap();
        assert_eq!(s.get_config("theme").unwrap(), Some("dark".into()));
    }

    #[test]
    fn get_missing_returns_none() {
        let s = test_store();
        assert_eq!(s.get_config("nonexistent").unwrap(), None);
    }

    #[test]
    fn set_overwrites() {
        let s = test_store();
        s.set_config("k", "v1").unwrap();
        s.set_config("k", "v2").unwrap();
        assert_eq!(s.get_config("k").unwrap(), Some("v2".into()));
    }

    #[test]
    fn delete_config() {
        let s = test_store();
        s.set_config("k", "v").unwrap();
        s.delete_config("k").unwrap();
        assert_eq!(s.get_config("k").unwrap(), None);
    }

    #[test]
    fn delete_nonexistent_is_noop() {
        let s = test_store();
        s.delete_config("nope").unwrap(); // no error
    }

    #[test]
    fn get_all_config() {
        let s = test_store();
        s.set_config("a", "1").unwrap();
        s.set_config("b", "2").unwrap();
        let all = s.get_all_config().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all["a"], "1");
        assert_eq!(all["b"], "2");
    }

    #[test]
    fn get_all_config_empty() {
        let s = test_store();
        assert_eq!(s.get_all_config().unwrap().len(), 0);
    }
}
