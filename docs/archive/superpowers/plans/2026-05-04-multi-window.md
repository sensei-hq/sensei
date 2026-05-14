# Multi-Window Design Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transition the Sensei desktop app from a single-window layout to a true Tauri multi-window model with a permanent Observatory/Collective window and per-project windows, backed by new DB bridge tables and scoped API endpoints.

**Architecture:** True OS-level multi-window via Tauri `WebviewWindow`. SvelteKit route groups `(observatory)` and `(project)` provide separate layouts. DB bridge tables (`project_libraries`, `extension_projects`) with `project_id = NULL` meaning global provide a unified scoping model with `scope` tag views.

**Tech Stack:** SvelteKit 2 · Svelte 5 runes · Tauri 2 · Rust/axum · PostgreSQL · sqlx · TypeScript

---

## Phase 1 — DB Schema

### Task 1: `project_libraries` bridge table DDL

**Files:**
- Create: `database/ddl/table/sensei/project_libraries.ddl`

- [ ] **Step 1: Write the DDL file**

```sql
CREATE TABLE IF NOT EXISTS sensei.project_libraries (
  library_id   uuid         NOT NULL REFERENCES sensei.libraries(id) ON DELETE CASCADE,
  project_id   uuid         REFERENCES sensei.projects(id) ON DELETE CASCADE,
  enabled      boolean      NOT NULL DEFAULT true,
  props        jsonb        NOT NULL DEFAULT '{}',
  modified_at  timestamptz  NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS project_libraries_global_uniq
    ON sensei.project_libraries(library_id)
 WHERE project_id IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS project_libraries_scoped_uniq
    ON sensei.project_libraries(library_id, project_id)
 WHERE project_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS project_libraries_project_id_idx
    ON sensei.project_libraries(project_id)
 WHERE enabled AND project_id IS NOT NULL;

COMMENT ON TABLE sensei.project_libraries IS
'Many-to-many: libraries associated with projects or marked global.
project_id NULL = global (available in every project).
project_id = X  = scoped to that project only.
Populated by daemon indexer from referenced_libraries; editable by user.';
```

- [ ] **Step 2: Commit**

```bash
git add database/ddl/table/sensei/project_libraries.ddl
git commit -m "feat(db): add project_libraries bridge table"
```

---

### Task 2: `extension_projects` bridge table DDL

**Files:**
- Create: `database/ddl/table/sensei/extension_projects.ddl`

- [ ] **Step 1: Write the DDL file**

```sql
CREATE TABLE IF NOT EXISTS sensei.extension_projects (
  extension_id  uuid         NOT NULL REFERENCES sensei.extensions(id) ON DELETE CASCADE,
  project_id    uuid         REFERENCES sensei.projects(id) ON DELETE CASCADE,
  enabled       boolean      NOT NULL DEFAULT true,
  props         jsonb        NOT NULL DEFAULT '{}',
  modified_at   timestamptz  NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS extension_projects_global_uniq
    ON sensei.extension_projects(extension_id)
 WHERE project_id IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS extension_projects_scoped_uniq
    ON sensei.extension_projects(extension_id, project_id)
 WHERE project_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS extension_projects_project_id_idx
    ON sensei.extension_projects(project_id)
 WHERE enabled AND project_id IS NOT NULL;

COMMENT ON TABLE sensei.extension_projects IS
'Many-to-many: extensions (skills/commands/agents/hooks) associated with projects or global.
project_id NULL = global (active in every project).
project_id = X  = active only in that project.
extensions.scope column is a seeding hint only; this table is authoritative at runtime.';
```

- [ ] **Step 2: Commit**

```bash
git add database/ddl/table/sensei/extension_projects.ddl
git commit -m "feat(db): add extension_projects bridge table"
```

---

### Task 3: `project_ftr_metrics` view DDL

**Files:**
- Create: `database/ddl/view/sensei/project_ftr_metrics.ddl`

- [ ] **Step 1: Write the DDL file**

```sql
CREATE OR REPLACE VIEW sensei.project_ftr_metrics AS
SELECT
  s.project_id,
  COUNT(*) FILTER (WHERE s.started_at > now() - interval '7d')   AS sessions_7d,
  AVG(CASE WHEN s.ftr THEN 1.0 ELSE 0.0 END)
    FILTER (WHERE s.started_at > now() - interval '14d')          AS ftr_14d,
  AVG(CASE WHEN s.ftr THEN 1.0 ELSE 0.0 END)
    FILTER (WHERE s.started_at > now() - interval '28d'
              AND s.started_at <= now() - interval '14d')         AS ftr_14d_prev
FROM activity.sessions s
WHERE s.project_id IS NOT NULL
GROUP BY s.project_id;
```

- [ ] **Step 2: Commit**

```bash
git add database/ddl/view/sensei/project_ftr_metrics.ddl
git commit -m "feat(db): add project_ftr_metrics view"
```

---

### Task 4: `project_drift` and `project_patterns` views DDL

**Files:**
- Create: `database/ddl/view/sensei/project_drift.ddl`
- Create: `database/ddl/view/sensei/project_patterns.ddl`

- [ ] **Step 1: Write project_drift.ddl**

```sql
CREATE OR REPLACE VIEW sensei.project_drift AS
SELECT di.*, f.project_id
  FROM inference.drift_items di
  JOIN sensei.folders f ON f.id = di.folder_id
 WHERE f.project_id IS NOT NULL;
```

- [ ] **Step 2: Write project_patterns.ddl**

```sql
CREATE OR REPLACE VIEW sensei.project_patterns AS
SELECT dp.*, f.project_id
  FROM inference.detected_patterns dp
  JOIN sensei.folders f ON f.id = dp.folder_id
 WHERE f.project_id IS NOT NULL;
```

- [ ] **Step 3: Commit**

```bash
git add database/ddl/view/sensei/project_drift.ddl database/ddl/view/sensei/project_patterns.ddl
git commit -m "feat(db): add project_drift and project_patterns views"
```

---

### Task 5: Resolved scope-tag views DDL

**Files:**
- Create: `database/ddl/view/sensei/project_libraries_resolved.ddl`
- Create: `database/ddl/view/sensei/project_extensions_resolved.ddl`

- [ ] **Step 1: Write project_libraries_resolved.ddl**

```sql
CREATE OR REPLACE VIEW sensei.project_libraries_resolved AS
SELECT
    l.*,
    pl.enabled,
    pl.props          AS project_props,
    pl.modified_at    AS associated_at,
    CASE
        WHEN pl.project_id IS NULL THEN 'global'
        ELSE 'project'
    END               AS scope,
    pl.project_id     AS scoped_project_id
FROM sensei.libraries l
JOIN sensei.project_libraries pl ON pl.library_id = l.id;
```

- [ ] **Step 2: Write project_extensions_resolved.ddl**

```sql
CREATE OR REPLACE VIEW sensei.project_extensions_resolved AS
SELECT
    e.*,
    ep.enabled,
    ep.props          AS project_props,
    ep.modified_at    AS associated_at,
    CASE
        WHEN ep.project_id IS NULL THEN 'global'
        ELSE 'project'
    END               AS scope,
    ep.project_id     AS scoped_project_id
FROM sensei.extensions e
JOIN sensei.extension_projects ep ON ep.extension_id = e.id;
```

- [ ] **Step 3: Apply schema**

```bash
dbd reset && dbd apply
```
Expected: All tables and views created without errors.

- [ ] **Step 4: Commit**

```bash
git add database/ddl/view/sensei/project_libraries_resolved.ddl database/ddl/view/sensei/project_extensions_resolved.ddl
git commit -m "feat(db): add project_libraries_resolved and project_extensions_resolved views with scope tag"
```

---

## Phase 2 — Daemon: pg_store methods

### Task 6: pg_store — project scope query methods

**Files:**
- Modify: `daemon/crates/senseid/src/db/pg_store.rs`

- [ ] **Step 1: Write the failing test**

Add at bottom of `pg_store.rs` in the `#[cfg(test)]` block:

```rust
#[sqlx::test]
async fn test_project_libraries_resolved(pool: PgPool) {
    let s = PgStore { pool };
    let proj_id = s.create_project("_test:lib_scope", None, None).await.unwrap();
    // Insert a library and a global bridge row
    let lib_id: (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.libraries(name, ecosystem) VALUES('test-lib','npm') RETURNING id"
    ).fetch_one(&s.pool).await.unwrap();
    sqlx_core::query::query(
        "INSERT INTO sensei.project_libraries(library_id, project_id) VALUES($1, NULL)"
    ).bind(lib_id.0).execute(&s.pool).await.unwrap();

    let libs = s.get_project_libraries(&proj_id).await.unwrap();
    assert_eq!(libs.len(), 1);
    assert_eq!(libs[0]["scope"].as_str(), Some("global"));
}
```

- [ ] **Step 2: Run test — verify it fails**

```bash
cd daemon && cargo test test_project_libraries_resolved -- --nocapture 2>&1 | tail -5
```
Expected: FAIL — `get_project_libraries` not found.

- [ ] **Step 3: Add `get_project_libraries` method to pg_store**

Find the `// ── Projects ──` section in `pg_store.rs` (around line 1047) and add after `delete_project`:

```rust
    pub async fn get_project_libraries(&self, project_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        // Query the resolved view directly — it already joins libraries internally
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, bool, serde_json::Value, String)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, ecosystem::text, description, enabled, project_props, scope
                 FROM sensei.project_libraries_resolved
                 WHERE (scoped_project_id = $1 OR scoped_project_id IS NULL)
                   AND enabled = true
                 ORDER BY scope DESC, name"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, name, ecosystem, desc, enabled, props, scope)| {
            serde_json::json!({
                "id": id, "name": name, "ecosystem": ecosystem,
                "description": desc, "enabled": enabled,
                "project_props": props, "scope": scope,
            })
        }).collect())
    }

    pub async fn get_project_extensions(&self, project_id: &uuid::Uuid, kind_filter: Option<&[&str]>) -> Result<Vec<serde_json::Value>, String> {
        // Query the resolved view directly — it already joins extensions internally
        let rows: Vec<(uuid::Uuid, String, String, bool, serde_json::Value, String)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, kind::text, enabled, project_props, scope
                 FROM sensei.project_extensions_resolved
                 WHERE (scoped_project_id = $1 OR scoped_project_id IS NULL)
                   AND enabled = true
                 ORDER BY scope DESC, name"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter()
            .filter(|(_, _, kind, _, _, _)| {
                kind_filter.map_or(true, |f| f.contains(&kind.as_str()))
            })
            .map(|(id, name, kind, enabled, props, scope)| {
                serde_json::json!({
                    "id": id, "name": name, "kind": kind,
                    "enabled": enabled, "project_props": props, "scope": scope,
                })
            }).collect())
    }

    pub async fn get_project_ftr(&self, project_id: &uuid::Uuid) -> Result<serde_json::Value, String> {
        let row: Option<(Option<f64>, Option<f64>, i64)> =
            sqlx_core::query_as::query_as(
                "SELECT ftr_14d, ftr_14d_prev, sessions_7d
                 FROM sensei.project_ftr_metrics WHERE project_id = $1"
            ).bind(project_id)
            .fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;

        let (ftr_14d, ftr_14d_prev, sessions_7d) = row.unwrap_or((None, None, 0));

        // 14-day daily trend array
        let daily: Vec<(chrono::NaiveDate, Option<f64>)> =
            sqlx_core::query_as::query_as(
                "SELECT date_trunc('day', started_at)::date AS day,
                        AVG(CASE WHEN ftr THEN 1.0 ELSE 0.0 END) AS daily_ftr
                 FROM activity.sessions
                 WHERE project_id = $1 AND started_at > now() - interval '14d'
                 GROUP BY day ORDER BY day"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        let trend: Vec<f64> = daily.into_iter().map(|(_, v)| v.unwrap_or(0.0)).collect();

        Ok(serde_json::json!({
            "ftr14d": ftr_14d.unwrap_or(0.0),
            "ftr14dPrev": ftr_14d_prev.unwrap_or(0.0),
            "ftrTrend": trend,
            "sessions7d": sessions_7d,
        }))
    }

    pub async fn get_project_drift(&self, project_id: &uuid::Uuid) -> Result<serde_json::Value, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, status::text, detail, detected_at
                 FROM sensei.project_drift WHERE project_id = $1
                 ORDER BY detected_at DESC LIMIT 200"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        let total = rows.len();
        let drifted = rows.iter().filter(|r| r.1 == "drifted").count();
        let broken = rows.iter().filter(|r| r.1 == "broken").count();
        let items: Vec<_> = rows.into_iter().map(|(id, status, detail, detected_at)| {
            serde_json::json!({ "id": id, "status": status, "detail": detail, "detectedAt": detected_at.to_rfc3339() })
        }).collect();

        Ok(serde_json::json!({ "items": items, "total": total, "drifted": drifted, "broken": broken }))
    }

    pub async fn get_project_patterns(&self, project_id: &uuid::Uuid) -> Result<serde_json::Value, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, bool, String, f64, i64)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, family, is_anti_pattern, lifecycle::text, confidence, instance_count
                 FROM sensei.project_patterns WHERE project_id = $1
                 ORDER BY is_anti_pattern, name"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        let (followed, anti): (Vec<_>, Vec<_>) = rows.into_iter().partition(|r| !r.3);
        let map_row = |(id, name, family, is_anti, lifecycle, confidence, count): (uuid::Uuid, String, Option<String>, bool, String, f64, i64)| {
            serde_json::json!({ "id": id, "name": name, "family": family, "isAntiPattern": is_anti, "lifecycle": lifecycle, "confidence": confidence, "instanceCount": count })
        };
        Ok(serde_json::json!({
            "followed": followed.into_iter().map(map_row).collect::<Vec<_>>(),
            "antiPatterns": anti.into_iter().map(map_row).collect::<Vec<_>>(),
        }))
    }

    pub async fn get_project_memories(&self, project_id: &uuid::Uuid) -> Result<serde_json::Value, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, f64, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, title, type::text, status::text, strength, last_relevant_at
                 FROM sensei.memories WHERE project_id = $1
                 ORDER BY last_relevant_at DESC LIMIT 100"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        let pending_share = rows.iter().filter(|r| r.3 == "pending_share").count();
        let active: Vec<_> = rows.into_iter()
            .filter(|r| r.3 == "active")
            .map(|(id, title, typ, status, strength, last)| {
                serde_json::json!({ "id": id, "title": title, "type": typ, "status": status, "strength": strength, "lastRelevantAt": last.to_rfc3339() })
            }).collect();

        Ok(serde_json::json!({ "active": active, "total": active.len(), "pendingShare": pending_share }))
    }

    pub async fn get_project_repos(&self, project_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, abs_path, kind::text FROM sensei.folders WHERE project_id = $1 ORDER BY name"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, name, path, kind)| {
            serde_json::json!({ "id": id, "name": name, "path": path, "kind": kind })
        }).collect())
    }

    pub async fn list_sessions_by_project(&self, project_id: &uuid::Uuid, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<bool>, String, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, task, ftr, outcome::text, started_at
                 FROM activity.sessions WHERE project_id = $1
                 ORDER BY started_at DESC LIMIT $2"
            ).bind(project_id).bind(limit)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, task, ftr, outcome, started)| {
            serde_json::json!({ "id": id, "task": task, "ftr": ftr, "outcome": outcome, "startedAt": started.to_rfc3339() })
        }).collect())
    }

    pub async fn get_project_recommendations(&self, project_id: &uuid::Uuid, status: Option<&str>) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, title, urgency::text, status::text, verdict::text
                 FROM inference.recommendations WHERE project_id = $1
                   AND ($2::text IS NULL OR status::text = $2)
                 ORDER BY urgency DESC, created_at DESC LIMIT 50"
            ).bind(project_id).bind(status)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, title, urgency, status, verdict)| {
            serde_json::json!({ "id": id, "title": title, "urgency": urgency, "status": status, "verdict": verdict })
        }).collect())
    }
```

- [ ] **Step 4: Run test — verify it passes**

```bash
cd daemon && cargo test test_project_libraries_resolved -- --nocapture 2>&1 | tail -5
```
Expected: PASS

- [ ] **Step 5: Run all daemon tests**

```bash
cd daemon && cargo test 2>&1 | tail -10
```
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add daemon/crates/senseid/src/db/pg_store.rs
git commit -m "feat(daemon): add project scope query methods to pg_store"
```

---

### Task 7: Daemon handlers — `project_detail.rs`

**Files:**
- Create: `daemon/crates/senseid/src/api/handlers/project_detail.rs`
- Modify: `daemon/crates/senseid/src/api/handlers/mod.rs`
- Modify: `daemon/crates/senseid/src/api/routes.rs`

- [ ] **Step 1: Write the failing test for `/projects/{id}/ftr`**

In `project_detail.rs` (new file), add a `#[cfg(test)]` block at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;
    use crate::api::state::AppState;
    use crate::db::pg_store::PgStore;

    async fn test_app() -> (TestServer, uuid::Uuid) {
        let pg = PgStore::connect_test().await.expect("test db");
        let project_id = pg.create_project("_test:detail", None, None).await.unwrap();
        let state = AppState::from_pg(pg);
        let router = crate::api::routes::create_router(state);
        (TestServer::new(router).unwrap(), project_id)
    }

    #[tokio::test]
    async fn test_get_project_ftr_returns_shape() {
        let (server, proj) = test_app().await;
        let resp = server.get(&format!("/api/projects/{}/ftr", proj)).await;
        resp.assert_status_ok();
        let body: serde_json::Value = resp.json();
        assert!(body["ftr14d"].is_number());
        assert!(body["sessions7d"].is_number());
        assert!(body["ftrTrend"].is_array());
    }
}
```

- [ ] **Step 2: Run test — verify it fails**

```bash
cd daemon && cargo test test_get_project_ftr_returns_shape -- --nocapture 2>&1 | tail -5
```
Expected: FAIL — module `project_detail` not found.

- [ ] **Step 3: Create `project_detail.rs` handler**

```rust
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use crate::api::state::AppState;

#[derive(Deserialize)]
pub(crate) struct RecoQuery {
    status: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct SessionsQuery {
    limit: Option<i64>,
}

pub(crate) async fn get_project_ftr(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let data = state.pg.get_project_ftr(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(data))
}

pub(crate) async fn get_project_repos(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let repos = state.pg.get_project_repos(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "repos": repos })))
}

pub(crate) async fn get_project_drift(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let data = state.pg.get_project_drift(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(data))
}

pub(crate) async fn get_project_patterns(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let data = state.pg.get_project_patterns(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(data))
}

pub(crate) async fn get_project_libraries(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let libs = state.pg.get_project_libraries(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "libraries": libs })))
}

pub(crate) async fn get_project_instruments(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let tools = state.pg.get_project_extensions(&uuid, Some(&["skill", "command", "agent"])).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "tools": tools })))
}

pub(crate) async fn get_project_memories(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let data = state.pg.get_project_memories(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(data))
}

pub(crate) async fn get_project_recommendations(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<RecoQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let recs = state.pg.get_project_recommendations(&uuid, q.status.as_deref()).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!(recs)))
}

pub(crate) async fn get_project_sessions(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<SessionsQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let limit = q.limit.unwrap_or(50);
    let sessions = state.pg.list_sessions_by_project(&uuid, limit).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "sessions": sessions })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::routing::get;
    use axum_test::TestServer;

    async fn make_server() -> (TestServer, uuid::Uuid) {
        let pg = crate::db::pg_store::PgStore::connect_test().await.expect("db");
        let project_id = pg.create_project("_test:detail_handler", None, None).await.unwrap();
        let state = crate::api::state::AppState::from_pg(pg);
        let router = Router::new()
            .route("/api/projects/:id/ftr", get(get_project_ftr))
            .route("/api/projects/:id/repos", get(get_project_repos))
            .route("/api/projects/:id/libraries", get(get_project_libraries))
            .route("/api/projects/:id/instruments", get(get_project_instruments))
            .with_state(state);
        (TestServer::new(router).unwrap(), project_id)
    }

    #[tokio::test]
    async fn test_get_project_ftr_returns_shape() {
        let (server, proj) = make_server().await;
        let resp = server.get(&format!("/api/projects/{}/ftr", proj)).await;
        resp.assert_status_ok();
        let body: serde_json::Value = resp.json();
        assert!(body["ftr14d"].is_number(), "missing ftr14d");
        assert!(body["sessions7d"].is_number(), "missing sessions7d");
        assert!(body["ftrTrend"].is_array(), "missing ftrTrend");
    }

    #[tokio::test]
    async fn test_get_project_repos_returns_array() {
        let (server, proj) = make_server().await;
        let resp = server.get(&format!("/api/projects/{}/repos", proj)).await;
        resp.assert_status_ok();
        assert!(resp.json::<serde_json::Value>()["repos"].is_array());
    }

    #[tokio::test]
    async fn test_get_project_libraries_returns_array() {
        let (server, proj) = make_server().await;
        let resp = server.get(&format!("/api/projects/{}/libraries", proj)).await;
        resp.assert_status_ok();
        assert!(resp.json::<serde_json::Value>()["libraries"].is_array());
    }
}
```

- [ ] **Step 4: Export module in `mod.rs`**

In `daemon/crates/senseid/src/api/handlers/mod.rs`, add:
```rust
pub(crate) mod project_detail;
```

- [ ] **Step 5: Wire routes in `routes.rs`**

In `routes.rs`, after the existing project routes block (`/api/projects/{id}/repos/{repo_id}`), add:

```rust
use crate::api::handlers::project_detail;

// Project detail endpoints
.route("/api/projects/{id}/ftr",             get(project_detail::get_project_ftr))
.route("/api/projects/{id}/repos",           get(project_detail::get_project_repos))
.route("/api/projects/{id}/drift",           get(project_detail::get_project_drift))
.route("/api/projects/{id}/patterns",        get(project_detail::get_project_patterns))
.route("/api/projects/{id}/libraries",       get(project_detail::get_project_libraries))
.route("/api/projects/{id}/instruments",     get(project_detail::get_project_instruments))
.route("/api/projects/{id}/memories",        get(project_detail::get_project_memories))
.route("/api/projects/{id}/recommendations", get(project_detail::get_project_recommendations))
.route("/api/projects/{id}/sessions",        get(project_detail::get_project_sessions))
```

- [ ] **Step 6: Run tests — verify they pass**

```bash
cd daemon && cargo test test_get_project -- --nocapture 2>&1 | tail -10
```
Expected: All 3 tests PASS.

- [ ] **Step 7: Build the daemon**

```bash
cd daemon && cargo build 2>&1 | tail -5
```
Expected: Compiles without errors.

- [ ] **Step 8: Commit**

```bash
git add daemon/crates/senseid/src/api/handlers/project_detail.rs daemon/crates/senseid/src/api/handlers/mod.rs daemon/crates/senseid/src/api/routes.rs
git commit -m "feat(daemon): add project detail API endpoints (ftr, repos, drift, patterns, libraries, instruments, memories, sessions)"
```

---

### Task 8: Update `api.ts` — add new project detail methods

**Files:**
- Modify: `app/src/lib/api.ts`

- [ ] **Step 1: Add methods to `senseiApi` in `api.ts`**

Find the line `listProjects: () => get...` block in `api.ts`. After `deleteProject`, add:

```typescript
    // ── Project detail (new multi-window endpoints) ───────────────────
    getProjectFtr: (id: string) =>
      get<{ ftr14d: number; ftr14dPrev: number; ftrTrend: number[]; sessions7d: number }>(
        `/api/projects/${enc(id)}/ftr`,
        { ftr14d: 0, ftr14dPrev: 0, ftrTrend: [], sessions7d: 0 }
      ),

    getProjectRepos: (id: string) =>
      get<{ repos: Array<{ id: string; name: string; path: string; kind: string }> }>(
        `/api/projects/${enc(id)}/repos`, { repos: [] }
      ),

    getProjectLibraries: (id: string) =>
      get<{ libraries: Array<{ id: string; name: string; ecosystem: string; scope: 'global' | 'project'; enabled: boolean }> }>(
        `/api/projects/${enc(id)}/libraries`, { libraries: [] }
      ),

    getProjectInstruments: (id: string) =>
      get<{ tools: Array<{ id: string; name: string; kind: string; scope: 'global' | 'project'; enabled: boolean }> }>(
        `/api/projects/${enc(id)}/instruments`, { tools: [] }
      ),

    getProjectMemories: (id: string) =>
      get<{ active: any[]; total: number; pendingShare: number }>(
        `/api/projects/${enc(id)}/memories`, { active: [], total: 0, pendingShare: 0 }
      ),

    getProjectDrift: (id: string) =>
      get<{ items: any[]; total: number; drifted: number; broken: number }>(
        `/api/projects/${enc(id)}/drift`, { items: [], total: 0, drifted: 0, broken: 0 }
      ),

    getProjectPatterns: (id: string) =>
      get<{ followed: any[]; antiPatterns: any[] }>(
        `/api/projects/${enc(id)}/patterns`, { followed: [], antiPatterns: [] }
      ),

    getProjectRecommendations: (id: string, status?: string) =>
      get<any[]>(
        `/api/projects/${enc(id)}/recommendations${status ? `?status=${status}` : ''}`, []
      ),

    getProjectSessions: (id: string, limit = 50) =>
      get<{ sessions: any[] }>(
        `/api/projects/${enc(id)}/sessions?limit=${limit}`, { sessions: [] }
      ),
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
cd app && bun run check 2>&1 | tail -10
```
Expected: No type errors in `api.ts`.

- [ ] **Step 3: Commit**

```bash
git add app/src/lib/api.ts
git commit -m "feat(app): add project detail API methods to senseiApi"
```

---

## Phase 3 — App: Route Restructure

### Task 9: Rename `(app)` → `(observatory)` route group

**Files:**
- Rename directory: `app/src/routes/(app)/` → `app/src/routes/(observatory)/`
- Modify: `app/src/routes/+page.svelte` (root redirect)

- [ ] **Step 1: Rename the directory**

```bash
mv app/src/routes/\(app\) app/src/routes/\(observatory\)
```

- [ ] **Step 2: Verify app builds**

```bash
cd app && bun run check 2>&1 | grep -E "Error|error" | head -10
```
Expected: No errors (SvelteKit resolves route groups by folder name, internal hrefs don't change).

- [ ] **Step 3: Update root page redirect if needed**

Read `app/src/routes/+page.svelte`. If it redirects to `/observatory`, it stays the same. If it renders anything else, no change needed.

- [ ] **Step 4: Commit**

```bash
git add -A app/src/routes/
git commit -m "refactor(app): rename (app) route group to (observatory)"
```

---

### Task 10: Create `(project)` route group scaffold

**Files:**
- Create: `app/src/routes/(project)/+layout.svelte`
- Create: `app/src/routes/(project)/project/[id]/+layout.svelte`
- Create: `app/src/routes/(project)/project/[id]/+layout.ts`
- Create: `app/src/routes/(project)/project/[id]/+page.svelte` (redirect to overview)
- Create: `app/src/routes/(project)/project/[id]/overview/+page.svelte`
- Create: `app/src/routes/(project)/project/[id]/sessions/+page.svelte`
- Create: `app/src/routes/(project)/project/[id]/memories/+page.svelte`
- Create: `app/src/routes/(project)/project/[id]/traceability/+page.svelte`
- Create: `app/src/routes/(project)/project/[id]/libraries/+page.svelte`
- Create: `app/src/routes/(project)/project/[id]/instruments/+page.svelte`
- Create: `app/src/routes/(project)/project/[id]/patterns/+page.svelte`
- Create: `app/src/routes/(project)/project/[id]/impact/+page.svelte`
- Create: `app/src/routes/(project)/project/[id]/about/+page.svelte`

- [ ] **Step 1: Create the (project) outer layout — bare shell**

`app/src/routes/(project)/+layout.svelte`:
```svelte
<script lang="ts">
  let { children } = $props();
</script>

<div class="project-window">
  {@render children()}
</div>

<style>
  .project-window {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }
</style>
```

- [ ] **Step 2: Create `project/[id]/+layout.ts` — load project data**

```typescript
import { appState } from '$lib/appstate.svelte.js';
import { senseiApi } from '$lib/api.js';

export async function load({ params }: { params: { id: string } }) {
  await appState.load();
  const api = senseiApi(appState.port);
  const [projects, ftrMetrics] = await Promise.all([
    api.listProjects(),
    api.getProjectFtr(params.id),
  ]);
  const project = projects.find((p: any) => p.id === params.id) ?? null;
  return { project, ftrMetrics, projectId: params.id };
}
```

- [ ] **Step 3: Create `project/[id]/+layout.svelte` — project chrome with sidebar**

```svelte
<script lang="ts">
  import { page } from '$app/stores';

  let { data, children } = $props();

  const SECTIONS = [
    { id: 'overview',      kanji: '観', label: 'Overview',      href: () => `/project/${data.projectId}/overview` },
    { id: 'sessions',      kanji: '刻', label: 'Sessions',      href: () => `/project/${data.projectId}/sessions` },
    { id: 'memories',      kanji: '憶', label: 'Memories',      href: () => `/project/${data.projectId}/memories` },
    { id: 'traceability',  kanji: '跡', label: 'Traceability',  href: () => `/project/${data.projectId}/traceability` },
    { id: 'libraries',     kanji: '書', label: 'Libraries',     href: () => `/project/${data.projectId}/libraries` },
    { id: 'instruments',   kanji: '具', label: 'Instruments',   href: () => `/project/${data.projectId}/instruments` },
    { id: 'patterns',      kanji: '型', label: 'Patterns',      href: () => `/project/${data.projectId}/patterns` },
    { id: 'impact',        kanji: '響', label: 'Impact',        href: () => `/project/${data.projectId}/impact` },
    { id: 'about',         kanji: '事', label: 'About',         href: () => `/project/${data.projectId}/about` },
  ];

  function isActive(sectionId: string): boolean {
    return $page.url.pathname.includes(`/${sectionId}`);
  }

  let kanji = $derived(data.project?.icon?.value ?? '場');
  let ftr = $derived(Math.round((data.ftrMetrics?.ftr14d ?? 0) * 100));
</script>

<div class="project-shell">
  <!-- 2px shu accent stripe (PerspectiveChrome) -->
  <div class="accent-stripe"></div>

  <!-- Titlebar / drag region -->
  <div class="titlebar drag-region">
    <span class="proj-kanji">{kanji}</span>
    <span class="proj-name">{data.project?.name ?? '…'}</span>
    <span class="proj-sub">· project window</span>
  </div>

  <div class="shell-body">
    <aside class="proj-sidebar">
      <div class="sidebar-stats">
        <span class="stat-value">{ftr}%</span>
        <span class="stat-label">FTR 14d</span>
      </div>

      <nav class="proj-nav">
        {#each SECTIONS as section (section.id)}
          {@const active = isActive(section.id)}
          <a href={section.href()} class="proj-nav-item" class:active>
            <span class="kanji">{section.kanji}</span>
            <span class="label">{section.label}</span>
          </a>
        {/each}
      </nav>
    </aside>

    <main class="proj-content">
      {@render children()}
    </main>
  </div>
</div>

<style>
  .project-shell { display: flex; flex-direction: column; height: 100vh; overflow: hidden; }
  .accent-stripe { height: 2px; background: var(--shu, #c0392b); flex-shrink: 0; }
  .titlebar { height: 36px; display: flex; align-items: center; gap: 8px; padding: 0 16px; flex-shrink: 0; }
  .proj-kanji { font-size: 18px; color: var(--shu, #c0392b); }
  .proj-name { font-size: 14px; font-weight: 600; }
  .proj-sub { font-size: 11px; opacity: 0.5; }
  .shell-body { display: flex; flex: 1; overflow: hidden; }
  .proj-sidebar { width: 180px; flex-shrink: 0; border-right: 1px solid var(--border); display: flex; flex-direction: column; padding: 12px 0; }
  .sidebar-stats { padding: 8px 16px 16px; }
  .stat-value { font-size: 22px; font-weight: 700; display: block; }
  .stat-label { font-size: 11px; opacity: 0.5; }
  .proj-nav { display: flex; flex-direction: column; }
  .proj-nav-item { display: flex; align-items: center; gap: 10px; padding: 7px 16px; text-decoration: none; color: inherit; font-size: 13px; }
  .proj-nav-item.active { background: var(--surface-2); color: var(--shu, #c0392b); }
  .proj-nav-item .kanji { width: 18px; text-align: center; }
  .proj-content { flex: 1; overflow-y: auto; }
</style>
```

- [ ] **Step 4: Create root project page redirect**

`app/src/routes/(project)/project/[id]/+page.svelte`:
```svelte
<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  onMount(() => goto(`/project/${$page.params.id}/overview`, { replaceState: true }));
</script>
```

- [ ] **Step 5: Create 9 stub section pages**

Each stub follows this pattern. Create all 9:

`app/src/routes/(project)/project/[id]/overview/+page.svelte`:
```svelte
<script lang="ts">
  let { data } = $props();
</script>
<div class="section-page">
  <h2>Overview — {data.project?.name}</h2>
  <p class="hint">FTR 14d: {Math.round((data.ftrMetrics?.ftr14d ?? 0) * 100)}%</p>
</div>
<style>
  .section-page { padding: 24px; }
  .hint { opacity: 0.5; font-size: 13px; }
</style>
```

`app/src/routes/(project)/project/[id]/sessions/+page.svelte`:
```svelte
<script lang="ts">
  let { data } = $props();
</script>
<div class="section-page"><h2>Sessions</h2><p class="hint">Project: {data.projectId}</p></div>
<style>.section-page { padding: 24px; } .hint { opacity: 0.5; font-size: 13px; }</style>
```

`app/src/routes/(project)/project/[id]/memories/+page.svelte`:
```svelte
<script lang="ts">
  let { data } = $props();
</script>
<div class="section-page"><h2>Memories</h2><p class="hint">Project: {data.projectId}</p></div>
<style>.section-page { padding: 24px; } .hint { opacity: 0.5; font-size: 13px; }</style>
```

`app/src/routes/(project)/project/[id]/traceability/+page.svelte`:
```svelte
<script lang="ts">
  let { data } = $props();
</script>
<div class="section-page"><h2>Traceability</h2><p class="hint">Project: {data.projectId}</p></div>
<style>.section-page { padding: 24px; } .hint { opacity: 0.5; font-size: 13px; }</style>
```

`app/src/routes/(project)/project/[id]/libraries/+page.svelte`:
```svelte
<script lang="ts">
  let { data } = $props();
</script>
<div class="section-page"><h2>Libraries</h2><p class="hint">Project: {data.projectId}</p></div>
<style>.section-page { padding: 24px; } .hint { opacity: 0.5; font-size: 13px; }</style>
```

`app/src/routes/(project)/project/[id]/instruments/+page.svelte`:
```svelte
<script lang="ts">
  let { data } = $props();
</script>
<div class="section-page"><h2>Instruments</h2><p class="hint">Project: {data.projectId}</p></div>
<style>.section-page { padding: 24px; } .hint { opacity: 0.5; font-size: 13px; }</style>
```

`app/src/routes/(project)/project/[id]/patterns/+page.svelte`:
```svelte
<script lang="ts">
  let { data } = $props();
</script>
<div class="section-page"><h2>Patterns</h2><p class="hint">Project: {data.projectId}</p></div>
<style>.section-page { padding: 24px; } .hint { opacity: 0.5; font-size: 13px; }</style>
```

`app/src/routes/(project)/project/[id]/impact/+page.svelte`:
```svelte
<script lang="ts">
  let { data } = $props();
</script>
<div class="section-page"><h2>Impact</h2><p class="hint">Project: {data.projectId}</p></div>
<style>.section-page { padding: 24px; } .hint { opacity: 0.5; font-size: 13px; }</style>
```

`app/src/routes/(project)/project/[id]/about/+page.svelte`:
```svelte
<script lang="ts">
  let { data } = $props();
</script>
<div class="section-page"><h2>About — {data.project?.name}</h2></div>
<style>.section-page { padding: 24px; }</style>
```

- [ ] **Step 6: Verify type-check passes**

```bash
cd app && bun run check 2>&1 | grep -E "Error|error" | head -10
```
Expected: No errors.

- [ ] **Step 7: Commit**

```bash
git add app/src/routes/\(project\)/
git commit -m "feat(app): scaffold (project) route group with 9 section stubs and layout"
```

---

## Phase 4 — App: Tauri Multi-Window

### Task 11: Window store and `openProjectWindow` utility

**Files:**
- Create: `app/src/lib/stores/windows.svelte.ts`

- [ ] **Step 1: Write the failing test**

Create `app/src/lib/stores/windows.spec.svelte.ts`:
```typescript
import { describe, it, expect, vi } from 'vitest';

// Mock Tauri window API before importing the store
vi.mock('@tauri-apps/api/window', () => ({
  WebviewWindow: vi.fn().mockImplementation(function(label: string) {
    this.label = label;
  }),
  getCurrentWindow: vi.fn().mockReturnValue({ label: 'test' }),
}));

import { openProjectWindow, openWindows } from './windows.svelte.js';

describe('openProjectWindow', () => {
  it('strips hyphens from project id for Tauri label', async () => {
    const { WebviewWindow } = await import('@tauri-apps/api/window');
    await openProjectWindow('abc-def-1234', 'My Project');
    const label = (WebviewWindow as any).mock.calls[0][0] as string;
    expect(label).toMatch(/^project-[a-zA-Z0-9]+$/);
    expect(label).not.toContain('-', 'project-'.length);
  });
});
```

- [ ] **Step 2: Run test — verify it fails**

```bash
cd app && bun run test windows.spec 2>&1 | tail -5
```
Expected: FAIL — module not found.

- [ ] **Step 3: Create `windows.svelte.ts`**

```typescript
import { WebviewWindow } from '@tauri-apps/api/window';

export interface OpenWindow {
  projectId: string;
  label: string;
  projectName: string;
}

let openWindowsState = $state<Map<string, OpenWindow>>(new Map());

export const openWindows = {
  get all(): OpenWindow[] { return [...openWindowsState.values()]; },
  has(projectId: string): boolean { return openWindowsState.has(projectId); },
};

export async function openProjectWindow(projectId: string, projectName: string): Promise<void> {
  const label = `project-${projectId.replace(/-/g, '')}`;

  // If window already open, bring to front
  const existing = WebviewWindow.getByLabel(label);
  if (existing) {
    await existing.setFocus();
    return;
  }

  openWindowsState.set(projectId, { projectId, label, projectName });

  const win = new WebviewWindow(label, {
    url: `/project/${projectId}`,
    title: `Sensei · ${projectName}`,
    width: 1200,
    height: 820,
    minWidth: 900,
    minHeight: 600,
    decorations: false,
  });

  win.once('tauri://destroyed', () => {
    openWindowsState.delete(projectId);
  });
}
```

- [ ] **Step 4: Run test — verify it passes**

```bash
cd app && bun run test windows.spec 2>&1 | tail -5
```
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add app/src/lib/stores/windows.svelte.ts app/src/lib/stores/windows.spec.svelte.ts
git commit -m "feat(app): add openProjectWindow utility and window state store"
```

---

### Task 12: Update Observatory layout — wire project click to `openProjectWindow`

**Files:**
- Modify: `app/src/routes/(observatory)/+layout.svelte`

- [ ] **Step 1: Read the current layout**

Read `app/src/routes/(observatory)/+layout.svelte` to confirm current project link structure (it uses `<a href="/projects/{proj.id}">`).

- [ ] **Step 2: Update NAV_ITEMS and project list**

Replace the `NAV_ITEMS` array and import block:

```svelte
<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import { openProjectWindow } from '$lib/stores/windows.svelte.js';

  let { children } = $props();

  const NAV_ITEMS = [
    { href: '/observatory', kanji: '家', label: 'Today' },
    { href: '/projects',    kanji: '場', label: 'Projects' },
    { href: '/sessions',    kanji: '刻', label: 'Sessions' },
    { href: '/insights',    kanji: '學', label: 'Insights' },
    { href: '/libraries',   kanji: '書', label: 'Libraries' },
    { href: '/instruments', kanji: '具', label: 'Instruments' },
  ];

  const BOTTOM_ITEMS = [
    { href: '/logs',     kanji: '録', label: 'Logs' },
    { href: '/settings', kanji: '設', label: 'Settings' },
  ];
```

- [ ] **Step 3: Replace project list items to call `openProjectWindow`**

Find the section that renders `<a href="/projects/{proj.id}">` and replace with:

```svelte
{#each projects as proj (proj.id)}
  <button
    type="button"
    class="nav-item proj-item"
    onclick={() => openProjectWindow(proj.id, proj.name)}
    title="{proj.name} ↗ opens in its own window"
  >
    <span class="kanji nav-kanji">{proj.kanji}</span>
    {#if !sidebarCollapsed}
      <span class="nav-label">{proj.name}</span>
      <span class="open-hint">↗</span>
    {/if}
  </button>
{/each}
```

- [ ] **Step 4: Add CSS for the open-hint**

Add to the layout's `<style>` block:
```css
.proj-item { background: none; border: none; cursor: pointer; width: 100%; text-align: left; color: inherit; }
.open-hint { font-size: 10px; opacity: 0.4; margin-left: auto; }
```

- [ ] **Step 5: Verify type-check passes**

```bash
cd app && bun run check 2>&1 | grep -E "Error|error" | head -10
```
Expected: No errors.

- [ ] **Step 6: Commit**

```bash
git add app/src/routes/\(observatory\)/+layout.svelte
git commit -m "feat(app): wire project sidebar items to openProjectWindow"
```

---

## Phase 5 — App: Wire Project Window Pages

### Task 13: Wire Overview page with real data

**Files:**
- Modify: `app/src/routes/(project)/project/[id]/overview/+page.svelte`
- Create: `app/src/routes/(project)/project/[id]/overview/+page.ts`

- [ ] **Step 1: Create `+page.ts` load function**

```typescript
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

export async function load({ params, parent }: any) {
  const { project, ftrMetrics } = await parent();
  await appState.load();
  const api = senseiApi(appState.port);
  const [reposData, recs, memoriesData, sessionsData] = await Promise.all([
    api.getProjectRepos(params.id),
    api.getProjectRecommendations(params.id, 'pending'),
    api.getProjectMemories(params.id),
    api.getProjectSessions(params.id, 4),
  ]);
  return {
    project,
    ftrMetrics,
    repos: reposData.repos,
    topRecommendation: recs[0] ?? null,
    memoryCount: memoriesData.total,
    memoriesPendingShare: memoriesData.pendingShare,
    recentSessions: sessionsData.sessions,
  };
}
```

- [ ] **Step 2: Update `+page.svelte` with wired data**

```svelte
<script lang="ts">
  let { data } = $props();
  let ftr = $derived(Math.round((data.ftrMetrics?.ftr14d ?? 0) * 100));
  let ftrPrev = $derived(Math.round((data.ftrMetrics?.ftr14dPrev ?? 0) * 100));
  let ftrDelta = $derived(ftr - ftrPrev);
</script>

<div class="overview-page">
  <!-- Hero: top recommendation -->
  {#if data.topRecommendation}
    <div class="hero-card">
      <span class="hero-label">Top recommendation</span>
      <p class="hero-title">{data.topRecommendation.title}</p>
      <span class="urgency-badge">{data.topRecommendation.urgency}</span>
    </div>
  {:else}
    <div class="hero-card empty">
      <p class="hint">No pending recommendations.</p>
    </div>
  {/if}

  <!-- Stat blocks -->
  <div class="stats-row">
    <div class="stat-block">
      <span class="stat-value">{ftr}%</span>
      <span class="stat-label">FTR 14d</span>
      {#if ftrDelta !== 0}
        <span class="stat-delta" class:pos={ftrDelta > 0} class:neg={ftrDelta < 0}>
          {ftrDelta > 0 ? '+' : ''}{ftrDelta}%
        </span>
      {/if}
    </div>
    <div class="stat-block">
      <span class="stat-value">{data.ftrMetrics?.sessions7d ?? 0}</span>
      <span class="stat-label">Sessions 7d</span>
    </div>
    <div class="stat-block">
      <span class="stat-value">{data.memoryCount}</span>
      <span class="stat-label">Memories</span>
      {#if data.memoriesPendingShare > 0}
        <span class="stat-badge">{data.memoriesPendingShare} to share</span>
      {/if}
    </div>
    <div class="stat-block">
      <span class="stat-value">{data.repos.length}</span>
      <span class="stat-label">Repos</span>
    </div>
  </div>

  <!-- Recent sessions -->
  {#if data.recentSessions.length > 0}
    <section class="recent-sessions">
      <h3 class="section-title">Recent sessions</h3>
      {#each data.recentSessions as session (session.id)}
        <div class="session-row">
          <span class="session-task">{session.task}</span>
          <span class="session-ftr" class:ftr-pass={session.ftr} class:ftr-fail={session.ftr === false}>
            {session.ftr === true ? '✓' : session.ftr === false ? '✗' : '—'}
          </span>
        </div>
      {/each}
    </section>
  {/if}
</div>

<style>
  .overview-page { padding: 24px; max-width: 800px; }
  .hero-card { background: var(--surface-2); border-radius: 8px; padding: 20px; margin-bottom: 20px; }
  .hero-card.empty { opacity: 0.5; }
  .hero-label { font-size: 11px; opacity: 0.6; display: block; margin-bottom: 6px; }
  .hero-title { font-size: 16px; font-weight: 600; margin: 0 0 8px; }
  .urgency-badge { font-size: 11px; padding: 2px 8px; border-radius: 10px; background: var(--surface-3); }
  .stats-row { display: flex; gap: 16px; margin-bottom: 24px; flex-wrap: wrap; }
  .stat-block { background: var(--surface-2); border-radius: 8px; padding: 16px; min-width: 100px; }
  .stat-value { font-size: 28px; font-weight: 700; display: block; }
  .stat-label { font-size: 11px; opacity: 0.5; display: block; }
  .stat-delta.pos { color: var(--green, green); font-size: 12px; }
  .stat-delta.neg { color: var(--red, red); font-size: 12px; }
  .stat-badge { font-size: 11px; background: var(--shu, #c0392b); color: white; padding: 2px 6px; border-radius: 8px; }
  .section-title { font-size: 13px; font-weight: 600; margin: 0 0 10px; opacity: 0.7; }
  .session-row { display: flex; justify-content: space-between; padding: 6px 0; border-bottom: 1px solid var(--border); font-size: 13px; }
  .ftr-pass { color: var(--green, green); }
  .ftr-fail { color: var(--red, red); }
  .hint { opacity: 0.5; font-size: 13px; }
</style>
```

- [ ] **Step 3: Verify type-check**

```bash
cd app && bun run check 2>&1 | grep -E "Error|error" | head -10
```
Expected: No errors.

- [ ] **Step 4: Commit**

```bash
git add app/src/routes/\(project\)/project/\[id\]/overview/
git commit -m "feat(app): wire project overview page with real data"
```

---

### Task 14: Wire Libraries page with scope badges

**Files:**
- Modify: `app/src/routes/(project)/project/[id]/libraries/+page.svelte`
- Create: `app/src/routes/(project)/project/[id]/libraries/+page.ts`

- [ ] **Step 1: Create `+page.ts`**

```typescript
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

export async function load({ params, parent }: any) {
  const { project } = await parent();
  await appState.load();
  const api = senseiApi(appState.port);
  const data = await api.getProjectLibraries(params.id);
  const wrappedCount = data.libraries.filter((l: any) => l.has_instruments).length;
  const unwrappedCount = data.libraries.length - wrappedCount;
  return { project, libraries: data.libraries, wrappedCount, unwrappedCount };
}
```

- [ ] **Step 2: Update `+page.svelte`**

```svelte
<script lang="ts">
  let { data } = $props();
</script>

<div class="libraries-page">
  <header class="page-header">
    <h2>Libraries</h2>
    <div class="header-stats">
      <span>{data.wrappedCount} wrapped</span>
      <span class="sep">·</span>
      <span>{data.unwrappedCount} unwrapped</span>
    </div>
  </header>

  {#if data.libraries.length === 0}
    <p class="empty-hint">No libraries associated with this project yet.</p>
  {:else}
    <ul class="lib-list">
      {#each data.libraries as lib (lib.id)}
        <li class="lib-row">
          <span class="lib-name">{lib.name}</span>
          <span class="lib-ecosystem">{lib.ecosystem}</span>
          <span class="scope-badge" class:global={lib.scope === 'global'} class:proj={lib.scope === 'project'}>
            [{lib.scope}]
          </span>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .libraries-page { padding: 24px; }
  .page-header { display: flex; align-items: baseline; gap: 16px; margin-bottom: 20px; }
  .page-header h2 { margin: 0; }
  .header-stats { font-size: 13px; opacity: 0.6; }
  .sep { opacity: 0.4; }
  .lib-list { list-style: none; margin: 0; padding: 0; }
  .lib-row { display: flex; align-items: center; gap: 10px; padding: 8px 0; border-bottom: 1px solid var(--border); font-size: 13px; }
  .lib-name { font-weight: 600; flex: 1; }
  .lib-ecosystem { opacity: 0.5; font-size: 12px; }
  .scope-badge { font-size: 11px; padding: 1px 6px; border-radius: 4px; font-family: monospace; }
  .scope-badge.global { background: var(--surface-3); opacity: 0.7; }
  .scope-badge.proj { background: color-mix(in srgb, var(--shu, #c0392b) 15%, transparent); color: var(--shu, #c0392b); }
  .empty-hint { opacity: 0.5; font-size: 13px; }
</style>
```

- [ ] **Step 3: Verify type-check**

```bash
cd app && bun run check 2>&1 | grep -E "Error|error" | head -10
```

- [ ] **Step 4: Commit**

```bash
git add app/src/routes/\(project\)/project/\[id\]/libraries/
git commit -m "feat(app): wire project libraries page with global/project scope badges"
```

---

### Task 15: Wire Instruments page with scope badges

**Files:**
- Modify: `app/src/routes/(project)/project/[id]/instruments/+page.svelte`
- Create: `app/src/routes/(project)/project/[id]/instruments/+page.ts`

- [ ] **Step 1: Create `+page.ts`**

```typescript
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

export async function load({ params, parent }: any) {
  const { project } = await parent();
  await appState.load();
  const api = senseiApi(appState.port);
  const data = await api.getProjectInstruments(params.id);
  return { project, tools: data.tools };
}
```

- [ ] **Step 2: Update `+page.svelte`**

```svelte
<script lang="ts">
  let { data } = $props();
</script>

<div class="instruments-page">
  <header class="page-header">
    <h2>Instruments</h2>
    <span class="total">{data.tools.length} tools</span>
  </header>

  {#if data.tools.length === 0}
    <p class="empty-hint">No instruments associated with this project yet.</p>
  {:else}
    <ul class="tool-list">
      {#each data.tools as tool (tool.id)}
        <li class="tool-row">
          <span class="tool-name">{tool.name}</span>
          <span class="tool-kind">{tool.kind}</span>
          <span class="scope-badge" class:global={tool.scope === 'global'} class:proj={tool.scope === 'project'}>
            [{tool.scope}]
          </span>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .instruments-page { padding: 24px; }
  .page-header { display: flex; align-items: baseline; gap: 16px; margin-bottom: 20px; }
  .page-header h2 { margin: 0; }
  .total { font-size: 13px; opacity: 0.6; }
  .tool-list { list-style: none; margin: 0; padding: 0; }
  .tool-row { display: flex; align-items: center; gap: 10px; padding: 8px 0; border-bottom: 1px solid var(--border); font-size: 13px; }
  .tool-name { font-weight: 600; flex: 1; }
  .tool-kind { opacity: 0.5; font-size: 12px; }
  .scope-badge { font-size: 11px; padding: 1px 6px; border-radius: 4px; font-family: monospace; }
  .scope-badge.global { background: var(--surface-3); opacity: 0.7; }
  .scope-badge.proj { background: color-mix(in srgb, var(--shu, #c0392b) 15%, transparent); color: var(--shu, #c0392b); }
  .empty-hint { opacity: 0.5; font-size: 13px; }
</style>
```

- [ ] **Step 3: Verify and commit**

```bash
cd app && bun run check 2>&1 | grep -E "Error|error" | head -5
git add app/src/routes/\(project\)/project/\[id\]/instruments/
git commit -m "feat(app): wire project instruments page with global/project scope badges"
```

---

### Task 16: Wire remaining project section pages

**Files:**
- Modify: `app/src/routes/(project)/project/[id]/sessions/+page.svelte` + `+page.ts`
- Modify: `app/src/routes/(project)/project/[id]/memories/+page.svelte` + `+page.ts`
- Modify: `app/src/routes/(project)/project/[id]/traceability/+page.svelte` + `+page.ts`
- Modify: `app/src/routes/(project)/project/[id]/patterns/+page.svelte` + `+page.ts`
- Modify: `app/src/routes/(project)/project/[id]/impact/+page.svelte` + `+page.ts`
- Modify: `app/src/routes/(project)/project/[id]/about/+page.svelte` + `+page.ts`

- [ ] **Step 1: Sessions `+page.ts`**

```typescript
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
export async function load({ params, parent }: any) {
  const { project, ftrMetrics } = await parent();
  await appState.load();
  const api = senseiApi(appState.port);
  const data = await api.getProjectSessions(params.id, 50);
  return { project, ftrMetrics, sessions: data.sessions };
}
```

Sessions `+page.svelte`:
```svelte
<script lang="ts">
  let { data } = $props();
</script>
<div class="section-page">
  <h2>Sessions</h2>
  <p class="stat">{data.ftrMetrics?.sessions7d ?? 0} in last 7 days</p>
  <ul class="session-list">
    {#each data.sessions as s (s.id)}
      <li class="session-row">
        <span class="task">{s.task}</span>
        <span class="ftr" class:pass={s.ftr} class:fail={s.ftr === false}>{s.ftr === true ? '✓' : s.ftr === false ? '✗' : '—'}</span>
        <span class="date">{new Date(s.startedAt).toLocaleDateString()}</span>
      </li>
    {/each}
  </ul>
</div>
<style>
  .section-page { padding: 24px; }
  .stat { font-size: 13px; opacity: 0.6; margin-bottom: 16px; }
  .session-list { list-style: none; margin: 0; padding: 0; }
  .session-row { display: flex; gap: 12px; padding: 8px 0; border-bottom: 1px solid var(--border); font-size: 13px; }
  .task { flex: 1; }
  .pass { color: var(--green, green); }
  .fail { color: var(--red, red); }
  .date { opacity: 0.5; font-size: 12px; }
</style>
```

- [ ] **Step 2: Memories `+page.ts`**

```typescript
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
export async function load({ params, parent }: any) {
  const { project } = await parent();
  await appState.load();
  const data = await senseiApi(appState.port).getProjectMemories(params.id);
  return { project, memories: data.active, pendingShare: data.pendingShare, total: data.total };
}
```

Memories `+page.svelte`:
```svelte
<script lang="ts">
  let { data } = $props();
</script>
<div class="section-page">
  <h2>Memories</h2>
  {#if data.pendingShare > 0}
    <div class="pending-banner">{data.pendingShare} memories pending collective share</div>
  {/if}
  <ul class="memory-list">
    {#each data.memories as m (m.id)}
      <li class="memory-row">
        <span class="memory-title">{m.title}</span>
        <span class="memory-type">{m.type}</span>
        <span class="memory-strength">{Math.round((m.strength ?? 0) * 100)}%</span>
      </li>
    {/each}
  </ul>
</div>
<style>
  .section-page { padding: 24px; }
  .pending-banner { background: color-mix(in srgb, var(--shu,#c0392b) 12%, transparent); color: var(--shu,#c0392b); padding: 10px 14px; border-radius: 6px; margin-bottom: 16px; font-size: 13px; }
  .memory-list { list-style: none; margin: 0; padding: 0; }
  .memory-row { display: flex; gap: 12px; padding: 8px 0; border-bottom: 1px solid var(--border); font-size: 13px; }
  .memory-title { flex: 1; }
  .memory-type { opacity: 0.5; font-size: 12px; }
  .memory-strength { font-size: 12px; font-family: monospace; }
</style>
```

- [ ] **Step 3: Traceability `+page.ts`**

```typescript
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
export async function load({ params, parent }: any) {
  const { project } = await parent();
  await appState.load();
  const data = await senseiApi(appState.port).getProjectDrift(params.id);
  return { project, driftItems: data.items, total: data.total, drifted: data.drifted, broken: data.broken };
}
```

Traceability `+page.svelte`:
```svelte
<script lang="ts">
  let { data } = $props();
</script>
<div class="section-page">
  <h2>Traceability</h2>
  <div class="drift-stats">
    <span>{data.total} tracked</span> · <span class="warn">{data.drifted} drifted</span> · <span class="danger">{data.broken} broken</span>
  </div>
  <ul class="drift-list">
    {#each data.driftItems as item (item.id)}
      <li class="drift-row" class:drifted={item.status === 'drifted'} class:broken={item.status === 'broken'}>
        <span class="status-dot"></span>
        <span class="detail">{item.detail ?? item.status}</span>
      </li>
    {/each}
  </ul>
</div>
<style>
  .section-page { padding: 24px; }
  .drift-stats { font-size: 13px; margin-bottom: 16px; opacity: 0.7; }
  .warn { color: var(--amber, orange); }
  .danger { color: var(--red, red); }
  .drift-list { list-style: none; margin: 0; padding: 0; }
  .drift-row { display: flex; gap: 10px; padding: 8px 0; border-bottom: 1px solid var(--border); font-size: 13px; }
  .drift-row.drifted .status-dot { background: var(--amber, orange); }
  .drift-row.broken .status-dot { background: var(--red, red); }
  .status-dot { width: 8px; height: 8px; border-radius: 50%; background: var(--green, green); margin-top: 4px; flex-shrink: 0; }
  .detail { flex: 1; }
</style>
```

- [ ] **Step 4: Patterns `+page.ts`**

```typescript
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
export async function load({ params, parent }: any) {
  const { project } = await parent();
  await appState.load();
  const data = await senseiApi(appState.port).getProjectPatterns(params.id);
  return { project, followed: data.followed, antiPatterns: data.antiPatterns };
}
```

Patterns `+page.svelte`:
```svelte
<script lang="ts">
  let { data } = $props();
</script>
<div class="section-page">
  <h2>Patterns</h2>
  {#if data.followed.length > 0}
    <section>
      <h3 class="sub">Followed ({data.followed.length})</h3>
      <ul class="pattern-list">
        {#each data.followed as p (p.id)}
          <li class="pattern-row"><span class="p-name">{p.name}</span><span class="p-lifecycle">{p.lifecycle}</span></li>
        {/each}
      </ul>
    </section>
  {/if}
  {#if data.antiPatterns.length > 0}
    <section>
      <h3 class="sub anti">Anti-patterns ({data.antiPatterns.length})</h3>
      <ul class="pattern-list">
        {#each data.antiPatterns as p (p.id)}
          <li class="pattern-row anti"><span class="p-name">{p.name}</span><span class="p-lifecycle">{p.lifecycle}</span></li>
        {/each}
      </ul>
    </section>
  {/if}
</div>
<style>
  .section-page { padding: 24px; }
  .sub { font-size: 12px; font-weight: 600; opacity: 0.6; margin: 16px 0 8px; text-transform: uppercase; letter-spacing: .05em; }
  .sub.anti { color: var(--red, red); }
  .pattern-list { list-style: none; margin: 0; padding: 0; }
  .pattern-row { display: flex; gap: 12px; padding: 7px 0; border-bottom: 1px solid var(--border); font-size: 13px; }
  .pattern-row.anti { opacity: 0.8; }
  .p-name { flex: 1; }
  .p-lifecycle { font-size: 11px; opacity: 0.5; font-family: monospace; }
</style>
```

- [ ] **Step 5: Impact `+page.ts`**

```typescript
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
export async function load({ params, parent }: any) {
  const { project } = await parent();
  await appState.load();
  const recs = await senseiApi(appState.port).getProjectRecommendations(params.id, 'accepted');
  return {
    project, verdicts: recs,
    positiveCount: recs.filter((r: any) => r.verdict === 'positive').length,
    negativeCount: recs.filter((r: any) => r.verdict === 'negative').length,
    pendingCount:  recs.filter((r: any) => r.verdict === 'pending').length,
  };
}
```

Impact `+page.svelte`:
```svelte
<script lang="ts">
  let { data } = $props();
</script>
<div class="section-page">
  <h2>Impact</h2>
  <div class="verdict-summary">
    <span class="pos">↑ {data.positiveCount}</span>
    <span class="neg">↓ {data.negativeCount}</span>
    <span class="pend">? {data.pendingCount}</span>
  </div>
  <ul class="verdict-list">
    {#each data.verdicts as v (v.id)}
      <li class="verdict-row" class:pos={v.verdict==='positive'} class:neg={v.verdict==='negative'}>
        <span class="v-title">{v.title}</span>
        <span class="v-urgency">{v.urgency}</span>
      </li>
    {/each}
  </ul>
</div>
<style>
  .section-page { padding: 24px; }
  .verdict-summary { display: flex; gap: 16px; font-size: 20px; font-weight: 700; margin-bottom: 20px; }
  .pos { color: var(--green, green); }
  .neg { color: var(--red, red); }
  .pend { opacity: 0.5; }
  .verdict-list { list-style: none; margin: 0; padding: 0; }
  .verdict-row { display: flex; gap: 12px; padding: 8px 0; border-bottom: 1px solid var(--border); font-size: 13px; }
  .verdict-row.pos { border-left: 3px solid var(--green, green); padding-left: 10px; }
  .verdict-row.neg { border-left: 3px solid var(--red, red); padding-left: 10px; }
  .v-title { flex: 1; }
  .v-urgency { opacity: 0.5; font-size: 12px; font-family: monospace; }
</style>
```

- [ ] **Step 6: About `+page.ts`**

```typescript
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
export async function load({ params, parent }: any) {
  const { project } = await parent();
  await appState.load();
  const reposData = await senseiApi(appState.port).getProjectRepos(params.id);
  return { project, repos: reposData.repos };
}
```

About `+page.svelte`:
```svelte
<script lang="ts">
  let { data } = $props();
  let p = $derived(data.project);
</script>
<div class="section-page">
  <h2>{p?.name ?? '—'}</h2>
  {#if p?.client}<p class="meta">Client: {p.client}</p>{/if}
  {#if p?.goal}<p class="goal">{p.goal}</p>{/if}
  <section>
    <h3 class="sub">Repos ({data.repos.length})</h3>
    <ul class="repo-list">
      {#each data.repos as repo (repo.id)}
        <li class="repo-row"><span class="repo-name">{repo.name}</span><span class="repo-path">{repo.path}</span></li>
      {/each}
    </ul>
  </section>
  {#if p?.stack}
    <section>
      <h3 class="sub">Stack</h3>
      <div class="stack-tags">
        {#each [...(p.stack.languages ?? []), ...(p.stack.frameworks ?? [])] as t}<span class="tag">{t}</span>{/each}
      </div>
    </section>
  {/if}
</div>
<style>
  .section-page { padding: 24px; max-width: 600px; }
  .meta, .goal { font-size: 13px; opacity: 0.7; margin: 4px 0; }
  .sub { font-size: 12px; font-weight: 600; opacity: 0.6; margin: 20px 0 8px; text-transform: uppercase; letter-spacing: .05em; }
  .repo-list { list-style: none; margin: 0; padding: 0; }
  .repo-row { display: flex; gap: 12px; padding: 6px 0; font-size: 13px; border-bottom: 1px solid var(--border); }
  .repo-name { font-weight: 600; }
  .repo-path { opacity: 0.5; font-size: 12px; font-family: monospace; overflow: hidden; text-overflow: ellipsis; }
  .stack-tags { display: flex; flex-wrap: wrap; gap: 6px; }
  .tag { background: var(--surface-3); font-size: 12px; padding: 3px 8px; border-radius: 4px; }
</style>
```

- [ ] **Step 7: Verify all type-check**

```bash
cd app && bun run check 2>&1 | grep -E "Error|error" | head -15
```
Expected: No errors.

- [ ] **Step 8: Commit**

```bash
git add app/src/routes/\(project\)/project/\[id\]/
git commit -m "feat(app): wire all 9 project window section pages with real data"
```

---

## Phase 6 — Observatory Pages: Projects Index & Insights Route

### Task 17: Create `/projects` full-page route and rename `/learnings` → `/insights`

**Files:**
- Create: `app/src/routes/(observatory)/projects/+page.svelte`
- Create: `app/src/routes/(observatory)/projects/+page.ts`
- Create: `app/src/routes/(observatory)/insights/+page.svelte` (rename from learnings)
- Delete: `app/src/routes/(observatory)/learnings/+page.svelte` (after copying content)

- [ ] **Step 1: Create `/projects` page load function**

`app/src/routes/(observatory)/projects/+page.ts`:
```typescript
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
export async function load() {
  await appState.load();
  const projects = await senseiApi(appState.port).listProjects();
  return { projects };
}
```

- [ ] **Step 2: Create `/projects` page**

`app/src/routes/(observatory)/projects/+page.svelte`:
```svelte
<script lang="ts">
  import { openProjectWindow } from '$lib/stores/windows.svelte.js';
  let { data } = $props();
</script>
<div class="projects-page">
  <h2>Projects</h2>
  {#if data.projects.length === 0}
    <p class="empty-hint">No projects yet. Set up a project to get started.</p>
  {:else}
    <div class="project-grid">
      {#each data.projects as proj (proj.id)}
        <button
          type="button"
          class="project-card"
          onclick={() => openProjectWindow(proj.id, proj.name)}
        >
          <span class="proj-kanji">{proj.icon?.value ?? '場'}</span>
          <span class="proj-name">{proj.name}</span>
          {#if proj.client}<span class="proj-client">{proj.client}</span>{/if}
          <span class="proj-maturity">{proj.maturity}</span>
          <span class="open-hint">↗</span>
        </button>
      {/each}
    </div>
  {/if}
</div>
<style>
  .projects-page { padding: 24px; }
  .project-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 16px; margin-top: 16px; }
  .project-card { background: var(--surface-2); border-radius: 10px; padding: 20px 16px; display: flex; flex-direction: column; gap: 6px; cursor: pointer; border: none; text-align: left; color: inherit; transition: background .15s; }
  .project-card:hover { background: var(--surface-3); }
  .proj-kanji { font-size: 32px; }
  .proj-name { font-size: 15px; font-weight: 700; }
  .proj-client { font-size: 12px; opacity: 0.6; }
  .proj-maturity { font-size: 11px; opacity: 0.4; font-family: monospace; }
  .open-hint { font-size: 10px; opacity: 0.3; margin-top: auto; }
  .empty-hint { opacity: 0.5; font-size: 13px; }
</style>
```

- [ ] **Step 3: Create `/insights` route (copy learnings content, update label)**

Read `app/src/routes/(observatory)/learnings/+page.svelte`, then create `app/src/routes/(observatory)/insights/+page.svelte` with the same content. Then delete the learnings file:

```bash
cp "app/src/routes/(observatory)/learnings/+page.svelte" "app/src/routes/(observatory)/insights/+page.svelte"
```

- [ ] **Step 4: Update sidebar nav items for new routes**

In `app/src/routes/(observatory)/+layout.svelte`, update `NAV_ITEMS` to use `/insights` instead of `/learnings` and add `/projects`:

```typescript
const NAV_ITEMS = [
  { href: '/observatory', kanji: '家', label: 'Today' },
  { href: '/projects',    kanji: '場', label: 'Projects' },
  { href: '/sessions',    kanji: '刻', label: 'Sessions' },
  { href: '/insights',    kanji: '學', label: 'Insights' },
  { href: '/libraries',   kanji: '書', label: 'Libraries' },
  { href: '/instruments', kanji: '具', label: 'Instruments' },
];
```

- [ ] **Step 5: Verify and commit**

```bash
cd app && bun run check 2>&1 | grep -E "Error|error" | head -5
git add app/src/routes/\(observatory\)/projects/ app/src/routes/\(observatory\)/insights/ app/src/routes/\(observatory\)/+layout.svelte
git commit -m "feat(app): add /projects full-page route, rename /learnings to /insights"
```

---

## Phase 7 — App Menu & Help

### Task 18: Register Tauri native app menu

**Files:**
- Modify: `daemon/crates/senseid/src/api/server.rs` (or `main.rs` — wherever Tauri app is built in `app/src-tauri/`)

- [ ] **Step 1: Find the Tauri app entry point**

```bash
find app/src-tauri -name "main.rs" -o -name "lib.rs" | head -5
```

- [ ] **Step 2: Add menu registration**

In the Tauri `main.rs` (or `lib.rs`), find the `tauri::Builder::default()` call. Add a menu build step before `.run()`:

```rust
use tauri::menu::{MenuBuilder, SubmenuBuilder};

// Inside the builder chain or setup hook:
.setup(|app| {
    let menu = MenuBuilder::new(app)
        .item(&SubmenuBuilder::new(app, "Sensei")
            .text("about", "About Sensei")
            .separator()
            .text("preferences", "Preferences…")
            .separator()
            .hide()
            .quit()
            .build()?)
        .item(&SubmenuBuilder::new(app, "File")
            .text("new-project", "New Project")
            .separator()
            .close_window()
            .build()?)
        .item(&SubmenuBuilder::new(app, "Edit")
            .undo()
            .redo()
            .separator()
            .cut()
            .copy()
            .paste()
            .select_all()
            .build()?)
        .item(&SubmenuBuilder::new(app, "View")
            .text("toggle-sidebar", "Toggle Sidebar")
            .build()?)
        .item(&SubmenuBuilder::new(app, "Window")
            .minimize()
            .zoom()
            .build()?)
        .item(&SubmenuBuilder::new(app, "Help")
            .text("shortcuts", "Keyboard Shortcuts")
            .text("whats-new", "What's New")
            .separator()
            .text("report-issue", "Report an Issue")
            .build()?)
        .build()?;
    app.set_menu(menu)?;

    app.on_menu_event(|app, event| {
        match event.id().as_ref() {
            "report-issue" => {
                let _ = tauri::opener::open_url("https://github.com/sensei-hq/sensei/issues", None::<&str>);
            }
            "shortcuts" => {
                // open help window — implement in Task 19
            }
            _ => {}
        }
    });
    Ok(())
})
```

- [ ] **Step 3: Add `tauri-plugin-opener` to Cargo.toml if not present**

```bash
grep -r "tauri-plugin-opener\|tauri::opener" app/src-tauri/ | head -3
```
If not present, add to `app/src-tauri/Cargo.toml`:
```toml
tauri-plugin-opener = "2"
```

- [ ] **Step 4: Build to verify**

```bash
cd app && bun tauri build --no-bundle 2>&1 | grep -E "error\[" | head -10
```
Expected: No compile errors.

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri/
git commit -m "feat(app): register native Tauri app menu with standard Mac items"
```

---

### Task 19: Create `/help` route

**Files:**
- Create: `app/src/routes/(observatory)/help/+page.svelte`

- [ ] **Step 1: Create the help page**

```svelte
<div class="help-page">
  <h1>Sensei Help</h1>

  <section>
    <h2>Quick Start</h2>
    <ol>
      <li><strong>Scan</strong> — Point Sensei at your project folders to index them.</li>
      <li><strong>Watch</strong> — Sensei observes your AI coding sessions and learns your patterns.</li>
      <li><strong>Teach</strong> — Adopt recommendations to improve your first-try rate.</li>
    </ol>
  </section>

  <section>
    <h2>Keyboard Shortcuts</h2>
    <table class="shortcuts">
      <tr><td>⌘N</td><td>New project</td></tr>
      <tr><td>⌘W</td><td>Close window</td></tr>
      <tr><td>⌘,</td><td>Preferences</td></tr>
      <tr><td>⌘\</td><td>Toggle sidebar</td></tr>
      <tr><td>⌘1</td><td>Today</td></tr>
      <tr><td>⌘2</td><td>Projects</td></tr>
      <tr><td>⌘3</td><td>Sessions</td></tr>
      <tr><td>⌘0</td><td>Show Observatory</td></tr>
      <tr><td>⌘?</td><td>This help window</td></tr>
    </table>
  </section>

  <section>
    <h2>FAQ</h2>
    <dl>
      <dt>What is FTR?</dt>
      <dd>First-Try Rate — the percentage of AI coding tasks completed without needing corrections. Higher is better.</dd>
      <dt>What is the daemon?</dt>
      <dd>A background process that indexes your repositories and watches for new sessions. It runs at <code>localhost:7749</code> by default.</dd>
      <dt>What does memory scope mean?</dt>
      <dd>Memories can be <em>project-scoped</em> (apply to one project) or shared to the <em>collective</em> (apply globally across all projects).</dd>
    </dl>
  </section>
</div>

<style>
  .help-page { padding: 32px; max-width: 640px; }
  h1 { font-size: 22px; margin-bottom: 24px; }
  h2 { font-size: 15px; font-weight: 700; margin: 28px 0 10px; }
  ol, ul { font-size: 14px; line-height: 1.7; padding-left: 20px; }
  .shortcuts { border-collapse: collapse; font-size: 13px; width: 100%; }
  .shortcuts td { padding: 5px 12px 5px 0; border-bottom: 1px solid var(--border); }
  .shortcuts td:first-child { font-family: monospace; font-size: 14px; font-weight: 600; min-width: 60px; }
  dl { font-size: 13px; }
  dt { font-weight: 700; margin-top: 12px; }
  dd { opacity: 0.7; margin: 4px 0 0 0; line-height: 1.6; }
  code { background: var(--surface-3); padding: 1px 5px; border-radius: 3px; font-size: 12px; }
</style>
```

- [ ] **Step 2: Verify and commit**

```bash
cd app && bun run check 2>&1 | grep -E "Error|error" | head -5
git add app/src/routes/\(observatory\)/help/
git commit -m "feat(app): add /help route with keyboard shortcuts and FAQ"
```

---

## Final Verification

- [ ] **Run full type-check**

```bash
cd app && bun run check 2>&1 | tail -5
```
Expected: 0 errors.

- [ ] **Run app unit tests**

```bash
cd app && bun run test 2>&1 | tail -10
```
Expected: All tests pass.

- [ ] **Run daemon tests**

```bash
cd daemon && cargo test 2>&1 | tail -10
```
Expected: All tests pass.

- [ ] **Start the app and verify project window opens**

```bash
cd app && bun run dev
```
Navigate to Observatory → click a project → verify a new window opens at `/project/{id}/overview` with the accent stripe and sidebar.

- [ ] **Merge to main**

```bash
git checkout main && git merge develop && git push
```
