use rusqlite::params;
use crate::types::IndexError;
use super::super::Store;

impl Store {
    #[allow(dead_code)]
    pub fn log_index_error(&self, err: &IndexError) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO index_errors(repo_id, file_path, error, adapter, timestamp) VALUES(?1, ?2, ?3, ?4, ?5)",
            params![err.repo_id, err.file_path, err.error, err.adapter, err.timestamp],
        )?;
        Ok(())
    }

    pub fn get_index_errors(&self, repo_id: Option<&str>) -> rusqlite::Result<Vec<IndexError>> {
        let map_row = |row: &rusqlite::Row| -> rusqlite::Result<IndexError> {
            Ok(IndexError {
                repo_id: row.get(0)?,
                file_path: row.get(1)?,
                error: row.get(2)?,
                adapter: row.get(3)?,
                timestamp: row.get(4)?,
            })
        };

        match repo_id {
            Some(id) => {
                let mut stmt = self.conn.prepare(
                    "SELECT repo_id, file_path, error, adapter, timestamp FROM index_errors WHERE repo_id = ?1 ORDER BY timestamp DESC"
                )?;
                stmt.query_map(params![id], map_row)?.collect()
            }
            None => {
                let mut stmt = self.conn.prepare(
                    "SELECT repo_id, file_path, error, adapter, timestamp FROM index_errors ORDER BY timestamp DESC LIMIT 200"
                )?;
                stmt.query_map([], map_row)?.collect()
            }
        }
    }

    pub fn clear_index_errors(&self, repo_id: &str) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM index_errors WHERE repo_id = ?1", params![repo_id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> Store { Store::open_memory().unwrap() }

    fn make_error(repo: &str, file: &str, err: &str, adapter: &str) -> IndexError {
        IndexError {
            repo_id: repo.into(), file_path: file.into(), error: err.into(),
            adapter: Some(adapter.into()), timestamp: "2026-04-12T12:00:00Z".into(),
        }
    }

    #[test]
    fn log_and_get_errors() {
        let s = test_store();
        s.log_index_error(&make_error("foo", "src/bad.ts", "SyntaxError", "typescript")).unwrap();
        s.log_index_error(&make_error("foo", "src/broken.py", "IndentationError", "python")).unwrap();
        s.log_index_error(&make_error("bar", "src/x.rs", "Parse error", "rust")).unwrap();

        assert_eq!(s.get_index_errors(None).unwrap().len(), 3);
        assert_eq!(s.get_index_errors(Some("foo")).unwrap().len(), 2);
        assert_eq!(s.get_index_errors(Some("bar")).unwrap().len(), 1);
    }

    #[test]
    fn clear_errors_for_repo() {
        let s = test_store();
        s.log_index_error(&make_error("foo", "a.ts", "err", "ts")).unwrap();
        s.log_index_error(&make_error("bar", "b.rs", "err", "rust")).unwrap();
        s.clear_index_errors("foo").unwrap();
        assert_eq!(s.get_index_errors(Some("foo")).unwrap().len(), 0);
        assert_eq!(s.get_index_errors(Some("bar")).unwrap().len(), 1);
    }

    #[test]
    fn get_errors_empty() {
        let s = test_store();
        assert_eq!(s.get_index_errors(None).unwrap().len(), 0);
        assert_eq!(s.get_index_errors(Some("nonexistent")).unwrap().len(), 0);
    }
}
