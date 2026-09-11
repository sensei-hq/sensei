use super::*;

/// Documentation served for a library, with the S9 verdict attached.
///
/// `version_note` is the load-bearing field: an answer for a version the caller
/// does not pin is useful LABELLED and wrong unlabelled (R4). `None` means the
/// answer matches the pin, or there was no pin to miss.
#[derive(Debug, Clone, Default)]
pub struct LibraryDocs {
    pub pages: Vec<serde_json::Value>,
    /// The version the served pages describe.
    pub served_version: Option<String>,
    /// What the asking folder pins, when it states one.
    pub pinned_version: Option<String>,
    /// The mismatch label, when the served version is not the pinned one.
    pub version_note: Option<String>,
}

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
impl PgStore {
    /// Search libraries by name (ILIKE).
    pub async fn search_libraries(&self, query: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, ecosystem::text, description FROM sensei.libraries
             WHERE name ILIKE '%' || $1 || '%'
             ORDER BY name LIMIT 50",
            )
            .bind(query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, eco, desc)| {
            serde_json::json!({ "id": id, "name": name, "ecosystem": eco, "description": desc })
        }).collect())
    }

    /// Get a single library by exact name.
    pub async fn get_library_by_name(&self, name: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, ecosystem::text, description FROM sensei.libraries
             WHERE name = $1
             ORDER BY name",
            )
            .bind(name)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, eco, desc)| {
            serde_json::json!({ "id": id, "name": name, "ecosystem": eco, "description": desc })
        }).collect())
    }

    /// Documentation pages for a library by name, optionally filtered to a
    /// single component. `component=None` returns every page (the handler
    /// builds the index/overview from these); `Some(c)` returns just that
    /// component's page(s). NULL-component pages (the library overview) sort
    /// first. This is what `get_lib_docs` reads — it must return the page
    /// CONTENT, not just library metadata.
    pub async fn get_library_pages(
        &self,
        name: &str,
        component: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        )> = sqlx_core::query_as::query_as(
            "SELECT c.name, c.component, c.description, c.body,
                        COALESCE(c.url, c.local_path) AS location, c.source_type::text,
                        c.package_name
                   FROM sensei.library_content c
                   JOIN sensei.library_versions lv ON lv.id = c.library_version_id AND lv.is_latest
                   JOIN sensei.libraries l ON l.id = lv.library_id
                  WHERE l.name = $1
                    AND c.kind = 'page'::sensei.library_content_kind
                    AND ($2::text IS NULL OR c.component = $2)
                  ORDER BY (c.component IS NULL) DESC, c.component, c.name",
        )
        .bind(name)
        .bind(component)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(title, component, description, content, location, source_type, package)| {
                serde_json::json!({
                    "title": title, "component": component,
                    "description": description, "content": content,
                    "location": location, "source": source_type,
                    // WHICH package this page documents. Null = library-level.
                    // A caller resolving `@rokkit/ui` picks the pages naming
                    // it and falls back to the library-level ones, instead of
                    // matching a component string against a symbol name.
                    "package": package,
                })
            })
            .collect())
    }

    /// Documentation for a library, CHOSEN and LABELLED against the version the
    /// asking folder actually pins (02b S9).
    ///
    /// `get_library_pages` serves whatever is marked latest. That is fine when
    /// the caller has no version in play and WRONG the moment they do: a
    /// project on 1.2 handed 3.0's docs gets a confident answer about an API it
    /// does not have. This resolves the pin from `referenced_libraries`, lets
    /// [`choose_docs`](crate::libraries::docs_source::choose_docs) pick the
    /// source that can actually serve it, and returns the label when it cannot.
    ///
    /// `folder_abs` is the asking folder. `None` means the caller stated no
    /// context — then there is no pin to mismatch against, the latest version
    /// is served, and NO label is invented.
    pub async fn get_library_docs(
        &self,
        name: &str,
        component: Option<&str>,
        folder_abs: Option<&str>,
    ) -> Result<LibraryDocs, String> {
        use crate::libraries::docs_source::{DocCandidate, DocRoute, choose_docs};

        let Some(lib_id) = self.library_id_for(name).await? else {
            return Ok(LibraryDocs::default());
        };

        // Every version we actually HOLD pages for. A version row with no
        // pages cannot serve anything, and offering it would be a choice the
        // caller could not use.
        let rows: Vec<(uuid::Uuid, String, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT v.id, v.version, v.source_type::text
               FROM sensei.library_versions v
              WHERE v.library_id = $1
                AND EXISTS (SELECT 1 FROM sensei.library_content c
                             WHERE c.library_version_id = v.id
                               AND c.kind = 'page'::sensei.library_content_kind)",
        )
        .bind(lib_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("get_library_docs: versions: {e}"))?;
        if rows.is_empty() {
            return Ok(LibraryDocs::default());
        }

        // The pin, from the asking folder. Absent context and absent pin are
        // the same thing here: nothing to compare against.
        let pinned: Option<String> = match folder_abs {
            Some(abs) => sqlx_core::query_as::query_as(
                "SELECT rl.version_used
                   FROM sensei.referenced_libraries rl
                   JOIN sensei.folders f ON f.id = rl.folder_id
                  WHERE rl.library_id = $1 AND f.abs_path = $2
                    AND rl.version_used IS NOT NULL AND rl.version_used <> ''",
            )
            .bind(lib_id)
            .bind(abs)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| format!("get_library_docs: pin: {e}"))?
            .map(|r: (String,)| r.0),
            None => None,
        };

        let (version_id, served_version, note) = match &pinned {
            Some(pin) => {
                let candidates: Vec<DocCandidate> = rows
                    .iter()
                    .map(|(_, version, st)| DocCandidate {
                        route: match st.as_deref() {
                            Some("llms.txt") => DocRoute::Website,
                            Some("http") => DocRoute::GitHub,
                            _ => DocRoute::Local,
                        },
                        version: Some(version.clone()),
                    })
                    .collect();
                let choice = choose_docs(pin, &candidates)
                    .ok_or_else(|| "get_library_docs: no candidate".to_string())?;
                // Map the chosen route+version back to its row.
                let picked = rows
                    .iter()
                    .find(|(_, v, st)| {
                        let route = match st.as_deref() {
                            Some("llms.txt") => DocRoute::Website,
                            Some("http") => DocRoute::GitHub,
                            _ => DocRoute::Local,
                        };
                        route == choice.route
                            && match &choice.fit {
                                crate::libraries::docs_source::VersionFit::Mismatch {
                                    serves,
                                    ..
                                } => v == serves,
                                _ => true,
                            }
                    })
                    .ok_or_else(|| "get_library_docs: choice did not map back".to_string())?;
                (picked.0, Some(picked.1.clone()), choice.fit.label())
            }
            // No pin: serve the latest, and do NOT invent a caveat.
            None => {
                let latest: Option<(uuid::Uuid, String)> = sqlx_core::query_as::query_as(
                    "SELECT id, version FROM sensei.library_versions
                      WHERE library_id = $1
                      ORDER BY is_latest DESC, modified_at DESC LIMIT 1",
                )
                .bind(lib_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| format!("get_library_docs: latest: {e}"))?;
                match latest {
                    Some((id, v)) => (id, Some(v), None),
                    None => return Ok(LibraryDocs::default()),
                }
            }
        };

        let pages: Vec<(
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        )> = sqlx_core::query_as::query_as(
            "SELECT c.name, c.component, c.description, c.body,
                        COALESCE(c.url, c.local_path), c.package_name
                   FROM sensei.library_content c
                  WHERE c.library_version_id = $1
                    AND c.kind = 'page'::sensei.library_content_kind
                    AND ($2::text IS NULL OR c.component = $2)
                  ORDER BY (c.component IS NULL) DESC, c.component, c.name",
        )
        .bind(version_id)
        .bind(component)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("get_library_docs: pages: {e}"))?;

        Ok(LibraryDocs {
            pages: pages
                .into_iter()
                .map(|(title, comp, desc, body, loc, pkg)| {
                    serde_json::json!({
                        "title": title, "component": comp, "description": desc,
                        "content": body, "location": loc, "package": pkg,
                    })
                })
                .collect(),
            served_version,
            pinned_version: pinned,
            version_note: note,
        })
    }

    /// Search library pages by title / component / content (ILIKE). Returns
    /// ranked matches with a short snippet rather than full content, so
    /// `search_lib_docs` is concise. Title/component hits rank above body hits.
    pub async fn search_library_pages(
        &self,
        query: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, String, Option<String>, Option<String>, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT l.name, c.name, c.component, c.description,
                        left(c.body, 400) AS snippet
                   FROM sensei.library_content c
                   JOIN sensei.library_versions lv ON lv.id = c.library_version_id AND lv.is_latest
                   JOIN sensei.libraries l ON l.id = lv.library_id
                  WHERE c.kind = 'page'::sensei.library_content_kind
                    AND (c.name ILIKE '%' || $1 || '%'
                      OR c.component ILIKE '%' || $1 || '%'
                      OR c.body ILIKE '%' || $1 || '%')
                  ORDER BY (c.name ILIKE '%' || $1 || '%') DESC,
                           (c.component ILIKE '%' || $1 || '%') DESC,
                           l.name, c.component
                  LIMIT 30",
            )
            .bind(query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(library, title, component, description, snippet)| {
                serde_json::json!({
                    "library": library, "title": title, "component": component,
                    "description": description, "snippet": snippet,
                })
            })
            .collect())
    }

    pub async fn upsert_library(
        &self,
        name: &str,
        ecosystem: &str,
        version: Option<&str>,
        description: Option<&str>,
        source_type: Option<&str>,
        base_url: Option<&str>,
    ) -> Result<uuid::Uuid, String> {
        // `libraries` is IDENTITY ONLY (S10 option B). `version`, `source_type`
        // and `base_url` describe a RELEASE, not a library, and stage 0 moved
        // them to `library_versions` — so this writes both levels rather than
        // four columns that no longer exist here.
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.libraries(name, ecosystem, description)
             VALUES($1, $2::sensei.library_ecosystem, $3)
             ON CONFLICT(ecosystem, name) DO UPDATE SET
               description = COALESCE(EXCLUDED.description, sensei.libraries.description),
               modified_at = now()
             RETURNING id",
        )
        .bind(name)
        .bind(ecosystem)
        .bind(description)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("upsert_library({name}): {e}"))?;
        let library_id = row.0;

        // The release half. COALESCE so a caller that knows only the name does
        // not wipe a source pointer an earlier resolution established.
        let version_id = self.ensure_library_version(&library_id, version).await?;
        sqlx_core::query::query(
            "UPDATE sensei.library_versions
                SET source_type = COALESCE($2::sensei.library_source_type, source_type),
                    base_url = COALESCE($3, base_url),
                    modified_at = now()
              WHERE id = $1",
        )
        .bind(version_id)
        .bind(source_type)
        .bind(base_url)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("upsert_library({name}): version: {e}"))?;

        Ok(library_id)
    }

    /// Refresh a library's source pointer (`source_type` + `base_url`) BY id — for
    /// a re-index that resolves the row via its uuid rather than by
    /// `(ecosystem, name)`. NEVER changes `ecosystem`: that is half the
    /// `upsert_library` conflict key and the row's identity, and clobbering it is
    /// exactly the phantom-row bug this avoids. `base_url` is COALESCE'd so a
    /// missing value doesn't wipe the stored one.
    ///
    /// Writes the CURRENT VERSION's pointer, not the library's: where a
    /// library's docs live is a property of the release that was fetched
    /// (S10 option B), and stage 0 moved both columns accordingly.
    pub async fn update_library_source(
        &self,
        id: &uuid::Uuid,
        source_type: &str,
        base_url: Option<&str>,
    ) -> Result<(), String> {
        let version_id = self.current_or_new_library_version(id).await?;
        sqlx_core::query::query(
            "UPDATE sensei.library_versions
                SET source_type = $2::sensei.library_source_type,
                    base_url = COALESCE($3, base_url),
                    modified_at = now()
              WHERE id = $1",
        )
        .bind(version_id)
        .bind(source_type)
        .bind(base_url)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("update_library_source: {e}"))?;
        Ok(())
    }

    /// Resolve — creating if absent — the `library_versions` row that a piece
    /// of library CONTENT belongs to (S7b).
    ///
    /// Content tables key on a VERSION, not a library, so every writer needs
    /// one. `version = None` means the caller does not know which version it
    /// fetched, and the honest key for that is `'unknown'` — NOT `'latest'`.
    ///
    /// `latest` is a TAG (`is_latest`), not a version, and using it as a key
    /// fabricates: at the S7b migration rokkit's 94 pages were keyed `latest`
    /// while its real version, 1.4.1, was sitting in the package.json on disk.
    /// A row keyed `unknown` states what we actually know and can be re-keyed
    /// the moment a version is resolved; a row keyed `latest` silently claims
    /// currency it was never given.
    ///
    /// This is a get-or-create ON PURPOSE, unlike `nodes.file_id` (R13), and
    /// the difference is which side owns the fact: a file row is created by
    /// the walk that observed the file, so a miss there means the pipeline is
    /// broken. A library version is observed by the fetch that is happening
    /// right now, so creating it here IS the observation.
    pub async fn ensure_library_version(
        &self,
        library_id: &uuid::Uuid,
        version: Option<&str>,
    ) -> Result<uuid::Uuid, String> {
        let key = version
            .map(str::trim)
            .filter(|v| !v.is_empty() && !v.eq_ignore_ascii_case("latest"))
            .unwrap_or("unknown");
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.library_versions(library_id, version, resolved_version, is_latest)
             VALUES($1, $2, $3, NOT EXISTS(
                 SELECT 1 FROM sensei.library_versions x
                  WHERE x.library_id = $1 AND x.is_latest))
             ON CONFLICT(library_id, version) DO UPDATE SET modified_at = now()
             RETURNING id",
        )
        .bind(library_id)
        .bind(key)
        .bind(version.filter(|v| !v.eq_ignore_ascii_case("latest")))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        self.refresh_latest_library_version(library_id).await?;
        Ok(row.0)
    }

    /// Re-elect a library's `is_latest` version by SEMVER, highest wins.
    ///
    /// The insert above seeds `is_latest` on whichever version arrived first,
    /// which makes "latest" mean "first seen" — and that is SCAN-ORDER
    /// DEPENDENT (R6). One repo pinning 1.0 and another pinning 2.0 would
    /// elect whichever was walked last, so every reader keyed on `is_latest`
    /// would return different docs depending on walk order.
    ///
    /// Ordering is done in Rust against the shared `parse_semver`, not in SQL:
    /// Postgres has no semver type, and a text sort puts `1.10.0` below
    /// `1.9.0`. Unparseable versions (`unknown`, a range) rank BELOW every
    /// real one and only win when nothing else is available — a placeholder
    /// must never outrank a concrete release.
    async fn refresh_latest_library_version(&self, library_id: &uuid::Uuid) -> Result<(), String> {
        let rows: Vec<(uuid::Uuid, String)> = sqlx_core::query_as::query_as(
            "SELECT id, version FROM sensei.library_versions WHERE library_id = $1",
        )
        .bind(library_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("refresh_latest_library_version: {e}"))?;

        // Max by (parseable, semver), then by version text so the choice is
        // total and repeatable even among unparseable values.
        let Some((winner, _)) = rows
            .iter()
            .map(|(id, v)| (*id, crate::libraries::version::parse_semver(v), v))
            .max_by(|a, b| {
                a.1.is_some()
                    .cmp(&b.1.is_some())
                    .then_with(|| a.1.cmp(&b.1))
                    .then_with(|| a.2.cmp(b.2))
            })
            .map(|(id, _, v)| (id, v))
        else {
            return Ok(());
        };

        // CLEAR then SET, in one transaction. A single
        // `SET is_latest = (id = $2)` violates the `library_versions_one_latest`
        // partial unique index: the index is checked per row, and Postgres
        // gives no ordering guarantee, so the new winner can be written while
        // the old one still holds the flag.
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        sqlx_core::query::query(
            "UPDATE sensei.library_versions SET is_latest = false
              WHERE library_id = $1 AND is_latest AND id <> $2",
        )
        .bind(library_id)
        .bind(winner)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("refresh_latest_library_version: clear: {e}"))?;
        sqlx_core::query::query(
            "UPDATE sensei.library_versions SET is_latest = true
              WHERE id = $1 AND NOT is_latest",
        )
        .bind(winner)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("refresh_latest_library_version: set: {e}"))?;
        tx.commit().await.map_err(|e| format!("refresh_latest_library_version: commit: {e}"))?;
        Ok(())
    }

    /// The version to READ when the caller named none — the single owner of
    /// that choice.
    ///
    /// Content hangs off a version (S10 option B), but almost every reader
    /// asks about a LIBRARY: "rokkit's skills", not "rokkit 1.4.1's skills".
    /// Something has to choose, and three copies of the rule is how
    /// `library_id_for` came to exist one concept over.
    ///
    /// `is_latest` first, then most recently touched. Falling back to
    /// `modified_at` matters because `is_latest` is only set when
    /// [`Self::ensure_library_version`] creates a library's FIRST version — a
    /// library whose rows arrived another way would otherwise read as having
    /// no content at all, which is the silent-empty failure rather than an
    /// honest one.
    ///
    /// The version to WRITE to when the caller names none.
    ///
    /// [`Self::current_library_version`] if the library has one, else a new
    /// `unknown` row. Writers must NOT call [`Self::ensure_library_version`]
    /// with `None` directly: that keys on the literal `unknown`, so a library
    /// whose real version was already recorded gets a SECOND row, content
    /// lands on it, and every reader — which picks the latest — finds nothing.
    /// That is not hypothetical; it is what broke the capability round-trip
    /// the moment `upsert_library` started creating a version of its own.
    pub async fn current_or_new_library_version(
        &self,
        library_id: &uuid::Uuid,
    ) -> Result<uuid::Uuid, String> {
        match self.current_library_version(library_id).await? {
            Some(id) => Ok(id),
            None => self.ensure_library_version(library_id, None).await,
        }
    }

    /// `None` means the library has no versions, which is a real state: known
    /// by name, nothing fetched yet.
    pub async fn current_library_version(
        &self,
        library_id: &uuid::Uuid,
    ) -> Result<Option<uuid::Uuid>, String> {
        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.library_versions
              WHERE library_id = $1
              ORDER BY is_latest DESC, modified_at DESC
              LIMIT 1",
        )
        .bind(library_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("current_library_version: {e}"))?;
        Ok(row.map(|r| r.0))
    }

    // ── Library capabilities (workstream D): skills/agents a library provides ──
    // Two writers coexist in one table, keyed by `source` ('manifest' | 'generated').

    /// Manifest-authoritative replace of a library's `source`-scoped capabilities:
    /// delete this library's rows for `source`, then re-insert — so a skill/agent
    /// REMOVED from a manifest disappears on re-ingest. One transaction. Mirrors
    /// [`Self::replace_folder_commands`]. `version_range` is the manifest's applies-to
    /// range (same for all rows). Only entries with a resolved `body` are persisted
    /// (a path/body-less entry is dropped upstream at ingest — no fabrication).
    /// Returns (skills, agents) written.
    pub async fn replace_library_capabilities(
        &self,
        library_id: &uuid::Uuid,
        source: &str,
        version_range: Option<&str>,
        skills: &[crate::libraries::manifest::ProvidedSkill],
        agents: &[crate::libraries::manifest::ProvidedAgent],
    ) -> Result<(u32, u32), String> {
        // Content hangs off a VERSION (S7b). `version_range` is an applies-to
        // RANGE, not a fetched version, so it is not the key — this content was
        // read from the manifest we have, which is 'latest' until a fetch says
        // otherwise.
        let version_id = self.current_or_new_library_version(library_id).await?;
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;

        // One DELETE where there were two, and it is scoped to the VERSION —
        // replacing a manifest must not wipe another release's capabilities.
        // `kind IN (...)` rather than unfiltered: a page fetched from the docs
        // route is not this manifest's to remove (R12 put all three in one
        // table; it did not make them one concern).
        sqlx_core::query::query(
            "DELETE FROM sensei.library_content
              WHERE library_version_id = $1 AND source = $2
                AND kind IN ('skill','agent')",
        )
        .bind(version_id)
        .bind(source)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        // Skills and agents differ only in `kind` and the two skill-only
        // columns, so one INSERT serves both. That IS the R12 payoff: the two
        // statements could previously drift, and the agent table's own comment
        // said it "mirrors library_skills".
        const UPSERT: &str = "INSERT INTO sensei.library_content(
                 library_version_id, kind, name, focus, body, source, source_path, version_range)
             VALUES($1, $2::sensei.library_content_kind, $3, $4, $5, $6, $7, $8)
             ON CONFLICT(library_version_id, kind, package_name, name) DO UPDATE SET
               focus=EXCLUDED.focus, body=EXCLUDED.body, source=EXCLUDED.source,
               source_path=EXCLUDED.source_path, version_range=EXCLUDED.version_range,
               modified_at=now()";

        let mut ns = 0u32;
        for s in skills.iter().filter(|s| s.body.is_some()) {
            sqlx_core::query::query(UPSERT)
                .bind(version_id)
                .bind("skill")
                .bind(&s.name)
                .bind(&s.focus)
                .bind(s.body.as_deref())
                .bind(source)
                .bind(s.path.as_deref())
                .bind(version_range)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
            ns += 1;
        }
        let mut na = 0u32;
        for a in agents.iter().filter(|a| a.body.is_some()) {
            sqlx_core::query::query(UPSERT)
                .bind(version_id)
                .bind("agent")
                .bind(&a.name)
                .bind(&a.focus)
                .bind(a.body.as_deref())
                .bind(source)
                .bind(a.path.as_deref())
                .bind(version_range)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
            na += 1;
        }
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok((ns, na))
    }

    /// Record where a library's source lives on disk.
    ///
    /// `libraries.local_path` had **no writer** — empty on all 1,121 rows — while
    /// manifest ingestion ran only inside `index_library` and only for a TRANSIENT
    /// `LocalDir` source. So nothing recorded where a manifest lived, nothing could
    /// re-read one, and rokkit's two newest capabilities (both present on disk)
    /// were never picked up. Writing this is what makes re-ingestion possible.
    pub async fn set_library_local_path(
        &self,
        library_id: &uuid::Uuid,
        local_path: &str,
    ) -> Result<(), String> {
        let version_id = self.current_or_new_library_version(library_id).await?;
        sqlx_core::query::query(
            "UPDATE sensei.library_versions SET local_path = $2, modified_at = now() WHERE id = $1",
        )
        .bind(version_id)
        .bind(local_path)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("set_library_local_path: {e}"))?;
        Ok(())
    }

    /// Libraries whose source location is known, as `(id, name, local_path)`.
    ///
    /// The set a manifest refresh can revisit. Empty until
    /// [`Self::set_library_local_path`] has run for something — which is exactly
    /// the state that made manifests un-re-readable.
    pub async fn libraries_with_local_path(
        &self,
    ) -> Result<Vec<(uuid::Uuid, String, String)>, String> {
        // DISTINCT ON the library: several versions of one library can each
        // carry a local_path, and a manifest refresh wants the library once,
        // at its current release — not the same library three times.
        sqlx_core::query_as::query_as(
            "SELECT DISTINCT ON (l.id) l.id, l.name, lv.local_path
               FROM sensei.libraries l
               JOIN sensei.library_versions lv ON lv.library_id = l.id
              WHERE lv.local_path IS NOT NULL AND lv.local_path <> ''
              ORDER BY l.id, lv.is_latest DESC, lv.modified_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("libraries_with_local_path: {e}"))
    }

    /// Resolve a library reference — its own NAME or one of its declared PACKAGE
    /// names — to a library id.
    ///
    /// The single owner of that resolution. Three readers need it
    /// ([`Self::list_library_skills`], [`Self::get_library_skill`],
    /// [`Self::list_library_agents`]), and three copies of a resolution rule is
    /// how the exclusion resolver came to gate the watcher while pruning nothing.
    ///
    /// The library's own name WINS over a package link. A library that publishes a
    /// package under its own name must resolve to itself, and a stale link must
    /// never shadow a real library.
    ///
    /// `None` on a genuine miss; propagates a read failure, because "no such
    /// library" and "the lookup broke" lead a caller to different places.
    pub async fn library_id_for(&self, reference: &str) -> Result<Option<uuid::Uuid>, String> {
        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT COALESCE(
                      (SELECT id FROM sensei.libraries WHERE name = $1),
                      (SELECT library_id FROM sensei.library_packages WHERE package_name = $1)
                    ) AS id
              WHERE COALESCE(
                      (SELECT id FROM sensei.libraries WHERE name = $1),
                      (SELECT library_id FROM sensei.library_packages WHERE package_name = $1)
                    ) IS NOT NULL",
        )
        .bind(reference)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("library_id_for({reference}): {e}"))?;
        Ok(row.map(|r| r.0))
    }

    /// Replace the packages a library declares it publishes.
    ///
    /// Whole-set and manifest-authoritative, exactly like
    /// [`Self::replace_library_capabilities`]: a package dropped from the manifest
    /// must stop resolving, and an incremental upsert would leave it pointing at
    /// this library forever.
    ///
    /// A package claimed by a DIFFERENT library is re-pointed rather than
    /// duplicated — the primary key is `package_name`, because a package belongs to
    /// exactly one library and two claims are a conflict, not two rows.
    pub async fn replace_library_packages(
        &self,
        library_id: &uuid::Uuid,
        source: &str,
        packages: &[String],
    ) -> Result<u64, String> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        sqlx_core::query::query(
            "DELETE FROM sensei.library_packages WHERE library_id = $1 AND source = $2",
        )
        .bind(library_id)
        .bind(source)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("replace_library_packages: clear: {e}"))?;

        let mut written = 0u64;
        for name in packages {
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            let r = sqlx_core::query::query(
                "INSERT INTO sensei.library_packages (package_name, library_id, source)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (package_name) DO UPDATE
                    SET library_id = EXCLUDED.library_id,
                        source      = EXCLUDED.source,
                        modified_at = now()",
            )
            .bind(name)
            .bind(library_id)
            .bind(source)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("replace_library_packages: insert {name}: {e}"))?;
            written += r.rows_affected();
        }
        tx.commit().await.map_err(|e| format!("replace_library_packages: commit: {e}"))?;
        Ok(written)
    }

    /// Skills a library provides, by its own NAME or one of its declared PACKAGE
    /// names (see [`Self::library_id_for`]). Enum-free; errors propagate.
    ///
    /// Resolved to an id first rather than joining on `l.name`, so `@rokkit/ui`
    /// reaches rokkit's skills. Two round trips, one owner of the resolution.
    pub async fn list_library_skills(
        &self,
        library: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        self.list_library_capabilities(library, "skill").await
    }

    /// Skills or agents a library provides — the shared body of
    /// [`Self::list_library_skills`] and [`Self::list_library_agents`].
    ///
    /// They were separate functions because they read separate tables. After
    /// R12 they differ by one literal, and leaving two copies would keep the
    /// drift risk the table collapse just removed.
    async fn list_library_capabilities(
        &self,
        library: &str,
        kind: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        let Some(lib_id) = self.library_id_for(library).await? else {
            return Ok(Vec::new());
        };
        let Some(version_id) = self.current_library_version(&lib_id).await? else {
            return Ok(Vec::new());
        };
        let rows: Vec<(String, Option<String>, Option<String>, String, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT c.name, c.focus, c.body, c.source, c.version_range
               FROM sensei.library_content c
              WHERE c.library_version_id = $1
                AND c.kind = $2::sensei.library_content_kind
              ORDER BY c.focus, c.name",
            )
            .bind(version_id)
            .bind(kind)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(name, focus, body, source, vr)| {
            serde_json::json!({ "name": name, "focus": focus, "body": body, "source": source, "version_range": vr })
        }).collect())
    }

    /// One skill of a library by `focus`. `focus` is NOT unique (uniqueness is on
    /// name), so this takes the most-recent match via `LIMIT 1` — never a multi-row
    /// error. `None` on a genuine miss (handler → 404), `Err` on failure.
    pub async fn get_library_skill(
        &self,
        library: &str,
        focus: &str,
    ) -> Result<Option<serde_json::Value>, String> {
        let Some(lib_id) = self.library_id_for(library).await? else {
            return Ok(None);
        };
        let Some(version_id) = self.current_library_version(&lib_id).await? else {
            return Ok(None);
        };
        let row: Option<(String, Option<String>, Option<String>, String, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT c.name, c.focus, c.body, c.source, c.version_range
               FROM sensei.library_content c
              WHERE c.library_version_id = $1
                AND c.kind = 'skill'::sensei.library_content_kind
                AND c.focus = $2
              ORDER BY c.modified_at DESC LIMIT 1",
            )
            .bind(version_id)
            .bind(focus)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(row.map(|(name, focus, body, source, vr)| {
            serde_json::json!({ "name": name, "focus": focus, "body": body, "source": source, "version_range": vr })
        }))
    }

    /// Review agents a library provides, by library NAME.
    pub async fn list_library_agents(
        &self,
        library: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        self.list_library_capabilities(library, "agent").await
    }

    /// The library skills/agents to SUGGEST for a project, from the libraries it
    /// depends on — REUSES `project_libraries_resolved` (the same view
    /// [`Self::get_library_enablement`] reads) joined to the capability tables. Backs
    /// the recommender enrichment. Returns `{suggested_skills, suggested_agents}`.
    pub async fn list_project_library_capabilities(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<serde_json::Value, String> {
        // One statement per kind, both now reaching content through the
        // VERSION level. `lv.is_latest` picks the release to suggest from —
        // without it a library with three fetched versions would suggest each
        // skill three times.
        const BY_KIND: &str = "SELECT pl.name, c.name, c.focus
               FROM sensei.project_libraries_resolved pl
               JOIN sensei.library_versions lv ON lv.library_id = pl.id AND lv.is_latest
               JOIN sensei.library_content c ON c.library_version_id = lv.id
              WHERE (pl.scoped_project_id = $1 OR pl.scoped_project_id IS NULL)
                AND pl.enabled = true
                AND c.kind = $2::sensei.library_content_kind
              ORDER BY pl.name, c.focus, c.name";
        let skills: Vec<(String, String, Option<String>)> = sqlx_core::query_as::query_as(BY_KIND)
            .bind(project_id)
            .bind("skill")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        let agents: Vec<(String, String, Option<String>)> = sqlx_core::query_as::query_as(BY_KIND)
            .bind(project_id)
            .bind("agent")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(serde_json::json!({
            "suggested_skills": skills.into_iter().map(|(lib, name, focus)| serde_json::json!({ "library": lib, "name": name, "focus": focus })).collect::<Vec<_>>(),
            "suggested_agents": agents.into_iter().map(|(lib, name, focus)| serde_json::json!({ "library": lib, "name": name, "focus": focus })).collect::<Vec<_>>(),
        }))
    }

    // ── Library update detection (workstream F, v0) ────────────────────────────

    /// Library pins per project, for the update scheduler: joins referenced_libraries
    /// (the folder's pinned `version_used`) → folders (project) → libraries. Returns
    /// `(library_id, name, ecosystem, local_path, project_id, version_used, base_url,
    /// source_type)`; only rows with a project and a non-empty pin. `base_url` +
    /// `source_type` let the apply arm rebuild the re-index `task.url` fail-closed.
    pub async fn list_library_project_pins(
        &self,
    ) -> Result<
        Vec<(
            uuid::Uuid,
            String,
            String,
            Option<String>,
            uuid::Uuid,
            String,
            Option<String>,
            Option<String>,
        )>,
        String,
    > {
        let rows = sqlx_core::query_as::query_as(
            // `local_path`, `base_url` and `source_type` describe a RELEASE, so
            // they come from `library_versions`. LEFT JOIN, because a library
            // referenced by a folder but never fetched has no version row —
            // and it is still a real pin the scheduler must see. An INNER join
            // would silently drop exactly the libraries with no docs yet.
            "SELECT l.id, l.name, l.ecosystem::text, lv.local_path, f.project_id,
                    rl.version_used, lv.base_url, lv.source_type::text
               FROM sensei.referenced_libraries rl
               JOIN sensei.libraries l ON l.id = rl.library_id
               JOIN sensei.folders f ON f.id = rl.folder_id
               LEFT JOIN LATERAL (
                    SELECT local_path, base_url, source_type
                      FROM sensei.library_versions v
                     WHERE v.library_id = l.id
                     ORDER BY v.is_latest DESC, v.modified_at DESC
                     LIMIT 1
               ) lv ON true
              WHERE f.project_id IS NOT NULL AND rl.version_used IS NOT NULL AND rl.version_used <> ''",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Cache the latest-known version + check time for a library in `libraries.props`
    /// (the TTL guard against re-hitting registries every tick). No schema change.
    pub async fn set_library_latest_cache(
        &self,
        library_id: &uuid::Uuid,
        latest: &str,
        checked_at_unix: i64,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.libraries
                SET props = coalesce(props, '{}'::jsonb)
                          || jsonb_build_object('latest_version', $2::text, 'latest_checked_at', $3::bigint)
              WHERE id = $1",
        )
        .bind(library_id).bind(latest).bind(checked_at_unix)
        .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// The cached `(latest_version, latest_checked_at_unix)` from `libraries.props`,
    /// if both are present.
    pub async fn get_library_latest_cache(
        &self,
        library_id: &uuid::Uuid,
    ) -> Result<Option<(String, i64)>, String> {
        let row: Option<(Option<String>, Option<i64>)> = sqlx_core::query_as::query_as(
            "SELECT props->>'latest_version', (props->>'latest_checked_at')::bigint FROM sensei.libraries WHERE id = $1",
        )
        .bind(library_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.and_then(|(v, t)| match (v, t) {
            (Some(v), Some(t)) => Some((v, t)),
            _ => None,
        }))
    }

    /// Stamp the "docs applied at version" marker in `libraries.props` after a
    /// CONFIRMED, non-empty re-index (F v1 auto-apply). Mirrors
    /// [`Self::set_library_latest_cache`]'s single-statement jsonb merge — no
    /// schema change. Only ever written on success, so it never fabricates
    /// "applied".
    pub async fn set_library_docs_applied(
        &self,
        library_id: &uuid::Uuid,
        version: &str,
        applied_at_unix: i64,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.libraries
                SET props = coalesce(props, '{}'::jsonb)
                          || jsonb_build_object('docs_applied_version', $2::text, 'docs_applied_at', $3::bigint)
              WHERE id = $1",
        )
        .bind(library_id).bind(version).bind(applied_at_unix)
        .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// The `docs_applied_version` marker from `libraries.props`, if present — the
    /// gate that stops the scheduler re-applying an already-applied version.
    pub async fn get_library_docs_applied(
        &self,
        library_id: &uuid::Uuid,
    ) -> Result<Option<String>, String> {
        let row: Option<(Option<String>,)> = sqlx_core::query_as::query_as(
            "SELECT props->>'docs_applied_version' FROM sensei.libraries WHERE id = $1",
        )
        .bind(library_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.and_then(|(v,)| v))
    }

    /// True if a recommendation already flags this project's update of `library_id`
    /// to `to_version` at the given security tier. `is_security` discriminates the
    /// tier so a prior/dismissed non-security notify can't suppress a later security
    /// flag (and vice-versa). Mirrors [`Self::recommendation_exists_for_pattern`],
    /// keyed on the library payload in `based_on`.
    pub async fn pending_library_update_exists(
        &self,
        project_id: &uuid::Uuid,
        library_id: &uuid::Uuid,
        to_version: &str,
        is_security: bool,
    ) -> Result<bool, String> {
        // The is_security discriminator: a row's tier is `based_on.is_security`
        // (absent/false = non-security). COALESCE the missing key to false so a
        // legacy notify (no key) reads as non-security, and only a same-tier row
        // matches — a non-security notify can't dedup-suppress a security flag.
        let row: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(
               SELECT 1 FROM inference.recommendations
                WHERE project_id = $1 AND action_type = 'library_update'
                  AND based_on->'library_update' @> jsonb_build_object('library_id', $2::text, 'to_version', $3::text)
                  AND COALESCE((based_on->'library_update'->>'is_security')::boolean, false) = $4)",
        )
        .bind(project_id).bind(library_id.to_string()).bind(to_version).bind(is_security)
        .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn get_library(&self, id: &uuid::Uuid) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, String, String, Option<String>, Option<String>, i32, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT l.id, l.name, l.ecosystem::text, lv.version, l.description,\n                        COALESCE(lv.page_count, 0), l.modified_at\n                   FROM sensei.libraries l\n                   LEFT JOIN LATERAL (
                        SELECT version, page_count
                          FROM sensei.library_versions v
                         WHERE v.library_id = l.id
                         ORDER BY v.is_latest DESC, v.modified_at DESC
                         LIMIT 1
                   ) lv ON true\n                  WHERE l.id = $1"
            ).bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(row.map(|(id, name, eco, ver, desc, pages, modified)| {
            serde_json::json!({
                "id": id, "name": name, "ecosystem": eco, "version": ver,
                "description": desc, "page_count": pages, "modified_at": modified.to_rfc3339(),
            })
        }))
    }

    pub async fn list_libraries(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, i32)> =
            sqlx_core::query_as::query_as(
                "SELECT l.id, l.name, l.ecosystem::text, lv.version, COALESCE(lv.page_count, 0)\n                   FROM sensei.libraries l\n                   LEFT JOIN LATERAL (
                        SELECT version, page_count
                          FROM sensei.library_versions v
                         WHERE v.library_id = l.id
                         ORDER BY v.is_latest DESC, v.modified_at DESC
                         LIMIT 1
                   ) lv ON true\n                  ORDER BY l.name"
            ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, name, eco, ver, pages)| {
            serde_json::json!({ "id": id, "name": name, "ecosystem": eco, "version": ver, "page_count": pages })
        }).collect())
    }

    /// List libraries joined with their folder usage. Returns one row per
    /// library with `repos` (folder names that reference it) and `repoCount`.
    /// Drives `GET /api/libs` for the setup wizard so the Libraries page can
    /// render ecosystem + version + usage without a second round-trip.
    pub async fn list_libraries_with_usage(
        &self,
        scope_folder_name: Option<&str>,
        scope_project_id: Option<&uuid::Uuid>,
        min_repos: i64,
    ) -> Result<Vec<serde_json::Value>, String> {
        // Aggregate by library, joining via referenced_libraries to count and
        // list distinct folder names. The optional scopes filter the *folders*
        // counted (not the library), so a lib appears only if some in-scope
        // folder references it.
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, Option<String>, i32, i64, Vec<String>)> =
            sqlx_core::query_as::query_as(
                // `version` and `page_count` live on the current VERSION now.
                // LEFT JOIN LATERAL, so a library referenced by folders but
                // never fetched still appears — it is exactly the row a reader
                // wants to see ("used in 9 repos, no docs held").
                "SELECT l.id, l.name, l.ecosystem::text, lv.version, l.description,
                        COALESCE(lv.page_count, 0) AS page_count,
                        COUNT(DISTINCT rl.folder_id)::bigint AS repo_count,
                        COALESCE(array_agg(DISTINCT f.name ORDER BY f.name), ARRAY[]::text[]) AS repos
                   FROM sensei.libraries l
                   JOIN sensei.referenced_libraries rl ON rl.library_id = l.id
                   JOIN sensei.folders f ON f.id = rl.folder_id
                   LEFT JOIN LATERAL (
                        SELECT version, page_count
                          FROM sensei.library_versions v
                         WHERE v.library_id = l.id
                         ORDER BY v.is_latest DESC, v.modified_at DESC
                         LIMIT 1
                   ) lv ON true
                  WHERE l.kind = 'detected'::sensei.library_kind
                    AND ($1::text     IS NULL OR f.name = $1)
                    AND ($2::uuid     IS NULL OR f.project_id = $2)
                  GROUP BY l.id, l.name, l.ecosystem, lv.version, l.description, lv.page_count
                 HAVING COUNT(DISTINCT rl.folder_id) >= $3
                  ORDER BY repo_count DESC, l.name"
            )
            .bind(scope_folder_name)
            .bind(scope_project_id)
            .bind(min_repos)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows
            .into_iter()
            .map(|(id, name, eco, ver, desc, pages, repo_count, repos)| {
                serde_json::json!({
                    "id": id, "name": name, "ecosystem": eco, "version": ver,
                    "description": desc, "pageCount": pages,
                    "repoCount": repo_count, "repos": repos,
                })
            })
            .collect())
    }

    pub async fn delete_library(&self, id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.libraries WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn upsert_library_page(
        &self,
        library_id: &uuid::Uuid,
        title: &str,
        url: Option<&str>,
        local_path: Option<&str>,
        description: Option<&str>,
        content: Option<&str>,
        source_type: &str,
        component: Option<&str>,
        package_name: Option<&str>,
    ) -> Result<uuid::Uuid, String> {
        // Pages hang off a VERSION (S7b). This writer is not told which one, so
        // it lands on the current release rather than a fabricated version.
        //
        // `package_name` says WHICH of the library's published packages this
        // page documents — `rokkit`'s List page is about `@rokkit/ui`, not
        // about rokkit as a whole. `None` means library-level (an overview, a
        // guide), which is a real state. Today every caller passes `None`
        // because no route reports a page's package yet; 02b S1's manifest
        // read is what will supply it. `None` is the honest "not stated",
        // never a guess from the page title.
        let version_id = self.current_or_new_library_version(library_id).await?;
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.library_content(
                 library_version_id, kind, name, url, local_path, description, body,
                 source_type, component, package_name, source, fetched_at)
             VALUES($1, 'page'::sensei.library_content_kind, $2, $3, $4, $5, $6,
                    $7::sensei.library_source_type, $8, $9, $7, now())
             ON CONFLICT(library_version_id, kind, package_name, name) DO UPDATE SET
               url = COALESCE(EXCLUDED.url, sensei.library_content.url),
               local_path = COALESCE(EXCLUDED.local_path, sensei.library_content.local_path),
               description = COALESCE(EXCLUDED.description, sensei.library_content.description),
               body = COALESCE(EXCLUDED.body, sensei.library_content.body),
               component = COALESCE(EXCLUDED.component, sensei.library_content.component),
               source_type = EXCLUDED.source_type,
               fetched_at = now(), modified_at = now()
             RETURNING id",
        )
        .bind(version_id)
        .bind(title)
        .bind(url)
        .bind(local_path)
        .bind(description)
        .bind(content)
        .bind(source_type)
        .bind(component)
        .bind(package_name)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Persist the URLs a registry response carried about a library (02b S8).
    ///
    /// COALESCE on every field: a later lookup that omits one must not wipe a
    /// value an earlier one established. Registries disagree about which URLs
    /// they expose — npm has no documentation field at all — so absence is
    /// "this response did not say", never "there is none".
    ///
    /// `repository_url` and `homepage_url` are identity-level and sit on
    /// `libraries`; `docs_url` describes where a RELEASE's documentation lives
    /// and sits on its version.
    pub async fn set_library_urls(
        &self,
        library_id: &uuid::Uuid,
        urls: &crate::libraries::registry::RegistryUrls,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.libraries
                SET repository_url = COALESCE($2, repository_url),
                    homepage_url   = COALESCE($3, homepage_url),
                    modified_at    = now()
              WHERE id = $1",
        )
        .bind(library_id)
        .bind(urls.repository.as_deref())
        .bind(urls.homepage.as_deref())
        .execute(&self.pool)
        .await
        .map_err(|e| format!("set_library_urls: {e}"))?;

        if let Some(docs) = urls.docs.as_deref() {
            let version_id = self.current_or_new_library_version(library_id).await?;
            sqlx_core::query::query(
                "UPDATE sensei.library_versions
                    SET docs_url = COALESCE($2, docs_url), modified_at = now()
                  WHERE id = $1",
            )
            .bind(version_id)
            .bind(docs)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("set_library_urls: docs: {e}"))?;
        }
        Ok(())
    }

    /// Record — or clear — why a library version has no current docs (02b S7b.2).
    ///
    /// `read_local_source_files` errors when the llms root holds no `.txt`
    /// files, and THAT ERROR IS THE STALENESS SIGNAL. dbd served 36 pages for
    /// two months pointing at a directory that had been deleted, and nothing
    /// registered it as a gap, because the failure only ever reached a task
    /// log. Recorded against the version, it is queryable:
    ///
    ///     SELECT * FROM sensei.library_versions WHERE props ? 'docs_error';
    ///
    /// Lives in `props` rather than a dedicated column: it is the documented
    /// extensible slot, and this is the `files.skip_detail` analogue — a
    /// verbatim reason, not a code anything branches on.
    ///
    /// `None` CLEARS it, so a library whose docs come back stops reporting
    /// stale without anyone intervening. `docs_checked_at` is written either
    /// way, because "checked and fine" and "never checked" are different
    /// states and only one of them is a gap.
    ///
    /// Does NOT touch the pages themselves (S7b.3). Stale content is the last
    /// known-true content; deleting it on a read error trades a stale answer
    /// for no answer.
    pub async fn record_library_docs_error(
        &self,
        library_id: &uuid::Uuid,
        error: Option<&str>,
    ) -> Result<(), String> {
        let version_id = self.current_or_new_library_version(library_id).await?;
        sqlx_core::query::query(
            "UPDATE sensei.library_versions
                SET props = CASE
                      WHEN $2::text IS NULL THEN (props - 'docs_error')
                      ELSE props || jsonb_build_object('docs_error', $2::text)
                    END || jsonb_build_object('docs_checked_at', now()),
                    modified_at = now()
              WHERE id = $1",
        )
        .bind(version_id)
        .bind(error)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("record_library_docs_error: {e}"))?;
        Ok(())
    }

    /// Refresh the denormalised page count on each of a library's versions.
    ///
    /// ONE statement, at the VERSION level. There used to be a second writing
    /// `libraries.page_count`, described as the across-all-versions total —
    /// but stage 0 moved the column off `libraries` entirely, so that write
    /// had been targeting a column that no longer exists. Summing versions to
    /// a library total would also be the wrong number: three fetched releases
    /// of one library do not mean three times the documentation.
    pub async fn update_library_page_count(&self, library_id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.library_versions v
                SET page_count = (SELECT count(*) FROM sensei.library_content c
                                   WHERE c.library_version_id = v.id
                                     AND c.kind = 'page'::sensei.library_content_kind),
                    modified_at = now()
              WHERE v.library_id = $1",
        )
        .bind(library_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("update_library_page_count: {e}"))?;
        Ok(())
    }

    /// Upsert a folder → library edge with optional `version_used` and `props`.
    ///
    /// `props` is merged (`||`) with any existing row's props, so callers can
    /// stack tags across passes without clobbering earlier metadata. Pass
    /// `None` for a props-free upsert.
    ///
    /// Typical `props` shape: `{"local_source": "../actions", "protocol": "link"}`
    /// for a dep declared via `link:` / `workspace:` / `file:` / Cargo `path=`.
    pub async fn upsert_referenced_library(
        &self,
        folder_id: &uuid::Uuid,
        library_id: &uuid::Uuid,
        version: Option<&str>,
        props: Option<serde_json::Value>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.referenced_libraries(folder_id, library_id, version_used, props)
             VALUES($1, $2, $3, COALESCE($4, '{}'::jsonb))
             ON CONFLICT(folder_id, library_id) DO UPDATE SET
               version_used = COALESCE(EXCLUDED.version_used, referenced_libraries.version_used),
               props = referenced_libraries.props || EXCLUDED.props,
               modified_at = now()",
        )
        .bind(folder_id)
        .bind(library_id)
        .bind(version)
        .bind(props)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Upsert a folder → folder edge into `sensei.folder_dependencies` (D11).
    ///
    /// Called from `extract_deps` when a `link:` / `workspace:` / `file:` /
    /// `path=` dep resolves to a sibling folder. Idempotent on the composite PK
    /// `(from_folder_id, to_folder_id, source_manifest)`.
    ///
    /// REGRAINED from a project → project edge, and that fixed a real gap: the
    /// old writer only recorded an edge when the two folders belonged to
    /// DIFFERENT projects, so a monorepo's crates depending on each other were
    /// silently dropped. Measured on this repo — all 384 folders share one
    /// project, so every `path=` dep between the 8 workspace members was
    /// discarded and the table held 0 rows while the feature was live and
    /// tested. A project-level answer is now a VIEW over this, derivable via
    /// `folders.project_id`.
    pub async fn upsert_folder_dependency(
        &self,
        from_folder_id: &uuid::Uuid,
        to_folder_id: &uuid::Uuid,
        source_protocol: &str,
        source_manifest: &str,
        resolved_target: Option<&str>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.folder_dependencies
                (from_folder_id, to_folder_id, source_protocol, source_manifest, resolved_target)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (from_folder_id, to_folder_id, source_manifest) DO UPDATE SET
               source_protocol = EXCLUDED.source_protocol,
               resolved_target = EXCLUDED.resolved_target,
               modified_at = now()",
        )
        .bind(from_folder_id)
        .bind(to_folder_id)
        .bind(source_protocol)
        .bind(source_manifest)
        .bind(resolved_target)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Roll a folder-level library reference up to a project-level association
    /// (sensei.library_enablement), scoped to `project_id`. `referenced_libraries`
    /// is folder-grained; `library_enablement` is the project↔library M2M the
    /// indexer owns and which `project_libraries_resolved` (the Projects screen)
    /// reads. Idempotent and non-destructive: `ON CONFLICT DO NOTHING` preserves
    /// any user edits to `enabled`/`props` on re-scan.
    pub async fn upsert_project_library(
        &self,
        library_id: &uuid::Uuid,
        project_id: &uuid::Uuid,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.library_enablement(library_id, project_id)
             VALUES($1, $2)
             ON CONFLICT (library_id, project_id) WHERE project_id IS NOT NULL DO NOTHING",
        )
        .bind(library_id)
        .bind(project_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Verdict measurement ────────────────────────────────────────────

    pub async fn get_library_usage(
        &self,
        library_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, String, Option<uuid::Uuid>, Option<String>, i64)> =
            sqlx_core::query_as::query_as(
                "SELECT library_name, folder, project_id, version_used, unresolved_import_count
             FROM sensei.library_usage WHERE library_id = $1 ORDER BY folder",
            )
            .bind(library_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(name, folder, pid, ver, imports)| {
                serde_json::json!({ "library_name": name, "folder": folder, "project_id": pid,
                                "version_used": ver, "import_count": imports })
            })
            .collect())
    }

    pub async fn get_library_enablement(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        // Query the resolved view directly — it already joins libraries internally.
        // Extended for T3 Slice 1.5: pull `page_count` (indexed docs marker) and
        // `local_path` (workspace / local-source marker) so the Libraries page
        // can render "wrapped by sensei" and "local source" badges without a
        // second round-trip.
        #[allow(clippy::type_complexity)]
        let rows: Vec<(
            uuid::Uuid,
            String,
            String,
            Option<String>,
            bool,
            serde_json::Value,
            String,
            i32,
            Option<String>,
        )> = sqlx_core::query_as::query_as(
            // `page_count` and `local_path` belong to a VERSION now, and the
            // view is `libraries.*` plus enablement — so they come from the
            // current release rather than from the view. LEFT JOIN: an enabled
            // library with nothing fetched yet is still enabled, and an INNER
            // join would drop it from the Projects screen entirely.
            "SELECT pl.id, pl.name, pl.ecosystem::text, pl.description, pl.enabled,
                        pl.project_props, pl.scope,
                        COALESCE(lv.page_count, 0), lv.local_path
                 FROM sensei.project_libraries_resolved pl
                 LEFT JOIN LATERAL (
                      SELECT page_count, local_path
                        FROM sensei.library_versions v
                       WHERE v.library_id = pl.id
                       ORDER BY v.is_latest DESC, v.modified_at DESC
                       LIMIT 1
                 ) lv ON true
                 WHERE (pl.scoped_project_id = $1 OR pl.scoped_project_id IS NULL)
                   AND pl.enabled = true
                 ORDER BY pl.scope DESC, pl.name",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(rows
            .into_iter()
            .map(|(id, name, ecosystem, desc, enabled, props, scope, page_count, local_path)| {
                serde_json::json!({
                    "id":            id,
                    "name":          name,
                    "ecosystem":     ecosystem,
                    "description":   desc,
                    "enabled":       enabled,
                    "project_props": props,
                    "scope":         scope,
                    "hasDocs":       page_count > 0,
                    "pageCount":     page_count,
                    "localSource":   local_path,
                })
            })
            .collect())
    }

    /// List libraries pinned to different versions across folders of a project.
    ///
    /// Reads `sensei.project_library_version_conflicts` — excludes local-
    /// protocol deps so only registry-version drift surfaces. Returns one row
    /// per conflicting (project, library) pair with the distinct versions and
    /// the folders where each version was seen.
    pub async fn list_project_library_version_conflicts(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Vec<String>, Vec<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT library_id, library_name, ecosystem, versions, folders
                   FROM sensei.project_library_version_conflicts
                  WHERE project_id = $1
                  ORDER BY library_name",
            )
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(rows
            .into_iter()
            .map(|(lib_id, name, ecosystem, versions, folders)| {
                serde_json::json!({
                    "library_id": lib_id,
                    "library_name": name,
                    "ecosystem": ecosystem,
                    "versions": versions,
                    "folders": folders,
                })
            })
            .collect())
    }

    /// List outgoing dependency edges for a project, DERIVED from the
    /// folder-grained `folder_dependencies` (D11).
    ///
    /// The stored fact is folder → folder; a project-level answer is this
    /// join, never a stored aggregate — a stored one can disagree with the
    /// manifest it came from. `to_project_id` may therefore be NULL (a target
    /// folder belonging to no project is still a real dependency), and an edge
    /// whose two folders sit in the SAME project is now included, having been
    /// silently dropped before the regrain.
    ///
    /// Returns one row per edge. Sorted for stable UI ordering.
    pub async fn list_project_dependencies(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(
            Option<uuid::Uuid>,
            Option<String>,
            uuid::Uuid,
            String,
            String,
            Option<String>,
            String,
        )> = sqlx_core::query_as::query_as(
            "SELECT to_p.id, to_p.name, fd.from_folder_id, fd.source_protocol,
                        fd.source_manifest, fd.resolved_target, from_f.name
                   FROM sensei.folder_dependencies fd
                   JOIN sensei.folders  from_f ON from_f.id = fd.from_folder_id
                   JOIN sensei.folders  to_f   ON to_f.id   = fd.to_folder_id
              LEFT JOIN sensei.projects to_p   ON to_p.id   = to_f.project_id
                  WHERE from_f.project_id = $1
                  ORDER BY to_p.name NULLS LAST, from_f.name, fd.source_manifest",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(rows
            .into_iter()
            .map(
                |(to_id, to_name, from_folder_id, protocol, manifest, target, from_folder_name)| {
                    serde_json::json!({
                        "to_project_id": to_id,
                        "to_project_name": to_name,
                        "from_folder_id": from_folder_id,
                        "from_folder": from_folder_name,
                        "source_protocol": protocol,
                        "source_manifest": manifest,
                        "resolved_target": target,
                    })
                },
            )
            .collect())
    }
}
