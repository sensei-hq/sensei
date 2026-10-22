//! The reads an ORM derivation needs beyond decoded symbols and call sites
//! (spec 20 §7c, #251): which type owns which member, what each type names as
//! its base, and which provider packages a workspace references.

use super::*;

impl PgStore {
    /// `(member fqn, owning node fqn)` for `language` in `folder_id` — the
    /// `nodes.parent_id` the indexer sets from ownership.
    pub async fn member_owners(
        &self,
        folder_id: &uuid::Uuid,
        language: &str,
    ) -> Result<Vec<(String, String)>, String> {
        sqlx_core::query_as::query_as(
            "SELECT c.fqn, p.fqn
               FROM sensei.nodes c
               JOIN sensei.nodes p ON p.id = c.parent_id
              WHERE c.folder_id = $1 AND c.language = $2
                AND c.fqn IS NOT NULL AND p.fqn IS NOT NULL",
        )
        .bind(folder_id)
        .bind(language)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("member owners: {e}"))
    }

    /// `(type fqn, base name)` for every `implements` / `extends` edge from
    /// `language` in `folder_id`: the placed target's name, or the name the
    /// source wrote when the edge is unplaced (an external `DbContext`).
    pub async fn type_bases(
        &self,
        folder_id: &uuid::Uuid,
        language: &str,
    ) -> Result<Vec<(String, String)>, String> {
        sqlx_core::query_as::query_as(
            "SELECT s.fqn, coalesce(t.name, e.target_name)
               FROM sensei.edges e
               JOIN sensei.nodes s ON s.id = e.source_id
               LEFT JOIN sensei.nodes t ON t.id = e.target_id
              WHERE s.folder_id = $1 AND s.language = $2 AND s.fqn IS NOT NULL
                AND e.kind::text IN ('implements', 'extends')
                AND coalesce(t.name, e.target_name) IS NOT NULL",
        )
        .bind(folder_id)
        .bind(language)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("type bases: {e}"))
    }

    /// `(folder abs_path, library name)` for the folders at or under
    /// `repo_abs` that reference one of `names`.
    pub async fn libraries_under(
        &self,
        repo_abs: &str,
        names: &[&str],
    ) -> Result<Vec<(String, String)>, String> {
        let names: Vec<String> = names.iter().map(|n| (*n).to_string()).collect();
        sqlx_core::query_as::query_as(
            "SELECT f.abs_path, l.name
               FROM sensei.referenced_libraries r
               JOIN sensei.folders f ON f.id = r.folder_id
               JOIN sensei.libraries l ON l.id = r.library_id
              WHERE (f.abs_path = $1 OR starts_with(f.abs_path, $1 || '/'))
                AND l.name = ANY($2)",
        )
        .bind(repo_abs)
        .bind(&names)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("libraries under {repo_abs}: {e}"))
    }
}
