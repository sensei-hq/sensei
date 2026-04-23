use rusqlite::params;
use super::super::Store;

impl Store {
    #[allow(clippy::too_many_arguments)]
    pub fn upsert_lib_doc(
        &self, id: &str, lib_name: &str, title: &str, url: Option<&str>,
        summary: &str, content: Option<&str>, source_type: &str,
        component: Option<&str>, indexed_at: &str,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO lib_docs(id, lib_name, title, url, summary, content, source_type, component, indexed_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![id, lib_name, title, url.unwrap_or(""), summary, content.unwrap_or(""), source_type, component.unwrap_or(""), indexed_at],
        )?;
        Ok(())
    }

    pub fn upsert_lib_meta(
        &self, name: &str, source_type: &str, base_url: Option<&str>,
        _version: Option<&str>, indexed_at: &str,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO lib_meta(name, source_type, base_url, indexed_at) VALUES(?1,?2,?3,?4)
             ON CONFLICT(name) DO UPDATE SET source_type=?2, base_url=COALESCE(?3, base_url), indexed_at=?4",
            params![name, source_type, base_url.unwrap_or(""), indexed_at],
        )?;
        Ok(())
    }

    pub fn get_lib_doc_component(&self, lib_name: &str, component: &str) -> rusqlite::Result<Option<crate::indexer::lib_indexer::LibDoc>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, url, summary, content, source_type, component, indexed_at FROM lib_docs WHERE lib_name = ?1 AND (component = ?2 OR component LIKE '%' || ?2 OR ((?2 = 'index' OR ?2 = 'README') AND component IN ('index', 'README', 'index.txt'))) ORDER BY length(component) ASC LIMIT 1"
        )?;
        let mut rows = stmt.query_map(params![lib_name, component], |row| {
            Ok(crate::indexer::lib_indexer::LibDoc {
                id: row.get(0)?, title: row.get(1)?, url: row.get(2)?, summary: row.get(3)?,
                content: row.get::<_, Option<String>>(4)?, source_type: row.get(5)?,
                component: row.get(6)?, indexed_at: row.get(7)?,
            })
        })?;
        match rows.next() {
            Some(Ok(doc)) => Ok(Some(doc)),
            _ => Ok(None),
        }
    }

    pub fn get_lib_docs(&self, lib_name: &str) -> rusqlite::Result<Vec<crate::indexer::lib_indexer::LibDoc>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, url, summary, content, source_type, component, indexed_at FROM lib_docs WHERE lib_name = ?1"
        )?;
        let rows = stmt.query_map(params![lib_name], |row| {
            Ok(crate::indexer::lib_indexer::LibDoc {
                id: row.get(0)?, title: row.get(1)?, url: row.get(2)?, summary: row.get(3)?,
                content: row.get::<_, Option<String>>(4)?, source_type: row.get(5)?,
                component: row.get(6)?, indexed_at: row.get(7)?,
            })
        })?;
        rows.collect()
    }

    pub fn search_lib_docs(&self, query: &str) -> rusqlite::Result<Vec<crate::indexer::lib_indexer::LibDoc>> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, title, url, summary, content, source_type, component, indexed_at FROM lib_docs WHERE title LIKE ?1 OR summary LIKE ?1 OR content LIKE ?1 LIMIT 20"
        )?;
        let rows = stmt.query_map(params![pattern], |row| {
            Ok(crate::indexer::lib_indexer::LibDoc {
                id: row.get(0)?, title: row.get(1)?, url: row.get(2)?, summary: row.get(3)?,
                content: row.get::<_, Option<String>>(4)?, source_type: row.get(5)?,
                component: row.get(6)?, indexed_at: row.get(7)?,
            })
        })?;
        rows.collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> Store { Store::open_memory().unwrap() }

    #[test]
    fn upsert_and_get_lib_docs() {
        let s = test_store();
        s.upsert_lib_doc("d1", "axum", "Router", Some("https://docs.rs/axum"), "Routing", Some("Full content"), "llms_txt", Some("router"), "2026-04-12").unwrap();
        let docs = s.get_lib_docs("axum").unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].title, "Router");
        assert_eq!(docs[0].component, "router");
    }

    #[test]
    fn get_lib_doc_component_exact() {
        let s = test_store();
        s.upsert_lib_doc("d1", "axum", "Router", None, "Routing", None, "llms_txt", Some("router"), "2026-04-12").unwrap();
        let doc = s.get_lib_doc_component("axum", "router").unwrap();
        assert!(doc.is_some());
        assert_eq!(doc.unwrap().title, "Router");
    }

    #[test]
    fn get_lib_doc_component_missing() {
        let s = test_store();
        assert!(s.get_lib_doc_component("axum", "nonexistent").unwrap().is_none());
    }

    #[test]
    fn search_lib_docs_by_title() {
        let s = test_store();
        s.upsert_lib_doc("d1", "axum", "Router Guide", None, "How to route", None, "llms_txt", None, "2026-04-12").unwrap();
        s.upsert_lib_doc("d2", "axum", "Middleware", None, "How to middleware", None, "llms_txt", None, "2026-04-12").unwrap();
        let results = s.search_lib_docs("Router").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Router Guide");
    }

    #[test]
    fn search_lib_docs_by_summary() {
        let s = test_store();
        s.upsert_lib_doc("d1", "zod", "Validation", None, "Schema validation library", None, "llms_txt", None, "2026-04-12").unwrap();
        let results = s.search_lib_docs("validation").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn upsert_lib_meta() {
        let s = test_store();
        s.upsert_lib_meta("axum", "llms_txt", Some("https://docs.rs/axum"), None, "2026-04-12").unwrap();
        // Second upsert updates fields
        s.upsert_lib_meta("axum", "api_docs", None, None, "2026-04-13").unwrap();
        // No crash, meta updated
    }

    #[test]
    fn get_lib_docs_empty() {
        let s = test_store();
        assert_eq!(s.get_lib_docs("nonexistent").unwrap().len(), 0);
    }

    #[test]
    fn search_lib_docs_empty() {
        let s = test_store();
        assert_eq!(s.search_lib_docs("nothing").unwrap().len(), 0);
    }
}
