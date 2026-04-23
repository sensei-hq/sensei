use rusqlite::params;
use crate::types::{Repo, Project};
use super::super::Store;

impl Store {
    pub fn list_projects(&self) -> rusqlite::Result<Vec<Project>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, client, category, created_at, updated_at FROM projects ORDER BY name"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                client: row.get(3)?,
                category: row.get(4)?,
                tags: vec![],
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        let mut projects: Vec<Project> = rows.collect::<Result<_, _>>()?;
        // Batch load tags
        let mut tag_stmt = self.conn.prepare(
            "SELECT entity_id, tag FROM tags WHERE entity_type = 'project' ORDER BY entity_id, tag"
        )?;
        let tag_rows = tag_stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut tag_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        for row in tag_rows.flatten() {
            tag_map.entry(row.0).or_default().push(row.1);
        }
        for p in &mut projects {
            p.tags = tag_map.remove(&p.id).unwrap_or_default();
        }
        Ok(projects)
    }

    /// Get repos belonging to a project.
    pub fn get_project_repos(&self, project_id: &str) -> rusqlite::Result<Vec<Repo>> {
        let all = self.list_repos()?;
        Ok(all.into_iter().filter(|r| r.project_id.as_deref() == Some(project_id)).collect())
    }

    pub fn create_project(&self, p: &Project) -> rusqlite::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR IGNORE INTO projects(id, name, description, client, category, created_at, updated_at) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![p.id, p.name, p.description, p.client, p.category, now, now],
        )?;
        Ok(())
    }

    pub fn delete_project(&self, id: &str) -> rusqlite::Result<()> {
        // Unassign repos from this project
        self.conn.execute("UPDATE repos SET project_id = NULL, role = 'unknown' WHERE project_id = ?1", params![id])?;
        self.conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        self.conn.execute("DELETE FROM tags WHERE entity_type = 'project' AND entity_id = ?1", params![id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> Store { Store::open_memory().unwrap() }

    fn make_project(id: &str, name: &str) -> Project {
        Project {
            id: id.into(), name: name.into(), description: None,
            client: None, category: "active".into(),
            tags: vec![], created_at: None, updated_at: None,
        }
    }

    fn seed_repo(s: &Store, id: &str) {
        s.upsert_repo_basic(id, id, &format!("/tmp/{}", id)).unwrap();
    }

    #[test]
    fn create_and_list_projects() {
        let s = test_store();
        s.create_project(&Project {
            id: "p1".into(), name: "Acme".into(),
            description: Some("Main".into()), client: Some("Corp".into()),
            category: "active".into(), tags: vec![], created_at: None, updated_at: None,
        }).unwrap();
        let projects = s.list_projects().unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "Acme");
        assert_eq!(projects[0].client, Some("Corp".into()));
    }

    #[test]
    fn project_with_repos() {
        let s = test_store();
        seed_repo(&s, "api");
        seed_repo(&s, "ui");
        s.create_project(&make_project("p1", "Platform")).unwrap();
        s.set_repo_project("api", "p1", "backend", Some("API")).unwrap();
        s.set_repo_project("ui", "p1", "frontend", None).unwrap();

        let repos = s.get_project_repos("p1").unwrap();
        assert_eq!(repos.len(), 2);
        assert_eq!(repos.iter().find(|r| r.repo_id == "api").unwrap().role, "backend");
    }

    #[test]
    fn delete_project_unassigns_repos() {
        let s = test_store();
        seed_repo(&s, "api");
        s.create_project(&make_project("p1", "Test")).unwrap();
        s.set_repo_project("api", "p1", "backend", None).unwrap();
        s.add_tag("project", "p1", "prod").unwrap();

        s.delete_project("p1").unwrap();
        assert_eq!(s.list_projects().unwrap().len(), 0);
        assert_eq!(s.get_tags("project", "p1").unwrap().len(), 0);
        let repo = s.get_repo("api").unwrap().unwrap();
        assert!(repo.project_id.is_none());
    }

    #[test]
    fn project_tags_loaded() {
        let s = test_store();
        s.create_project(&make_project("p1", "Test")).unwrap();
        s.add_tag("project", "p1", "production").unwrap();
        let projects = s.list_projects().unwrap();
        assert_eq!(projects[0].tags, vec!["production"]);
    }

    #[test]
    fn duplicate_create_is_noop() {
        let s = test_store();
        s.create_project(&make_project("p1", "First")).unwrap();
        s.create_project(&make_project("p1", "Second")).unwrap(); // INSERT OR IGNORE
        let projects = s.list_projects().unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "First"); // original preserved
    }

    #[test]
    fn get_project_repos_empty() {
        let s = test_store();
        s.create_project(&make_project("p1", "Empty")).unwrap();
        assert_eq!(s.get_project_repos("p1").unwrap().len(), 0);
    }
}
