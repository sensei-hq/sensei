use super::*;

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
impl PgStore {
    /// Read the single collective-preferences row, or `None` when unset (the API
    /// then returns conservative defaults). `categories` comes back as raw jsonb.
    pub async fn get_collective_preferences(&self) -> Result<Option<CollectivePrefsRow>, String> {
        let row: Option<(String, String, serde_json::Value, String, String)> =
            sqlx_core::query_as::query_as(
                "SELECT destination, cadence, categories, attribution_default, updated_at::text
                   FROM sensei.collective_preferences WHERE singleton = true",
            )
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(row.map(|(destination, cadence, categories, attribution_default, updated_at)| {
            CollectivePrefsRow { destination, cadence, categories, attribution_default, updated_at }
        }))
    }

    /// Upsert the single collective-preferences row (keys on the `singleton` PK)
    /// and return the new `updated_at`. Callers validate the enum fields first.
    pub async fn set_collective_preferences(
        &self,
        destination: &str,
        cadence: &str,
        categories: &serde_json::Value,
        attribution_default: &str,
    ) -> Result<String, String> {
        let (updated_at,): (String,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.collective_preferences
                (singleton, destination, cadence, categories, attribution_default, updated_at)
             VALUES (true, $1, $2, $3, $4, now())
             ON CONFLICT (singleton) DO UPDATE SET
                destination         = EXCLUDED.destination,
                cadence             = EXCLUDED.cadence,
                categories          = EXCLUDED.categories,
                attribution_default = EXCLUDED.attribution_default,
                updated_at          = now()
             RETURNING updated_at::text",
        )
        .bind(destination)
        .bind(cadence)
        .bind(categories)
        .bind(attribution_default)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(updated_at)
    }

    // ── Insight copy cache (narration-cache pipeline) ────────────────────

    /// List memory-share batches for a project, newest first. `only_status`
    /// filters to a single lifecycle stage (`proposed`, `approved`, …); pass
    /// `None` to include every stage.
    pub async fn list_memory_share_batches(
        &self,
        project_id: &uuid::Uuid,
        only_status: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>, i64)> =
            sqlx_core::query_as::query_as(
                "SELECT b.id, b.status::text, b.note, b.created_at, b.decided_at,
                        (SELECT count(*) FROM sensei.memory_share_batch_members m WHERE m.batch_id = b.id)::bigint
                   FROM sensei.memory_share_batches b
                  WHERE b.project_id = $1
                    AND ($2::text IS NULL OR b.status::text = $2)
                  ORDER BY b.created_at DESC
                  LIMIT 200"
            )
            .bind(project_id)
            .bind(only_status)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(rows
            .into_iter()
            .map(|(id, status, note, created_at, decided_at, member_count)| {
                serde_json::json!({
                    "id":          id,
                    "status":      status,
                    "note":        note,
                    "createdAt":   created_at.to_rfc3339(),
                    "decidedAt":   decided_at.map(|t| t.to_rfc3339()),
                    "memberCount": member_count,
                })
            })
            .collect())
    }

    /// Create a new `proposed` memory-share batch with the given memory ids.
    /// Rejects an empty member list — a batch with nothing to share is a
    /// caller-side bug. Returns the new batch id on success.
    pub async fn create_memory_share_batch(
        &self,
        project_id: &uuid::Uuid,
        memory_ids: &[uuid::Uuid],
        note: Option<&str>,
    ) -> Result<uuid::Uuid, String> {
        if memory_ids.is_empty() {
            return Err("memory_ids must be non-empty".into());
        }
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;

        let (batch_id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.memory_share_batches (project_id, note)
             VALUES ($1, $2) RETURNING id",
        )
        .bind(project_id)
        .bind(note)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        // ON CONFLICT DO NOTHING is intentional — the composite PK guards
        // against duplicate members when a caller passes the same id twice.
        sqlx_core::query::query(
            "INSERT INTO sensei.memory_share_batch_members (batch_id, memory_id)
             SELECT $1, unnest($2::uuid[])
             ON CONFLICT DO NOTHING",
        )
        .bind(batch_id)
        .bind(memory_ids)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(batch_id)
    }

    /// Set a memory-share batch's terminal status. Accepts `approved`,
    /// `rejected`, or `withdrawn`. `approved` / `rejected` stamp
    /// `decided_at = now()`; `withdrawn` clears it (the batch was never
    /// decided). Errors when the batch is missing or already decided.
    pub async fn set_memory_share_batch_status(
        &self,
        batch_id: &uuid::Uuid,
        new_status: &str,
        note: Option<&str>,
    ) -> Result<(), String> {
        if !matches!(new_status, "approved" | "rejected" | "withdrawn") {
            return Err(format!("invalid status {new_status}"));
        }
        let decided_at_sql = if new_status == "withdrawn" { "NULL" } else { "now()" };
        let sql = format!(
            "UPDATE sensei.memory_share_batches
                SET status = $1::sensei.memory_share_batch_status,
                    note = COALESCE($2, note),
                    decided_at = {decided_at_sql}
              WHERE id = $3
                AND status = 'proposed'"
        );
        let result = sqlx_core::query::query(&sql)
            .bind(new_status)
            .bind(note)
            .bind(batch_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        if result.rows_affected() == 0 {
            return Err("batch not found or already decided".into());
        }
        Ok(())
    }

    // ── Dōjō upstream contribute (C6) ─────────────────────────────────

    /// Load a share batch's `(project_id, status, member items)` for the C6
    /// upstream-contribute path. `status` is returned so the caller can enforce
    /// "only `approved` batches contribute". Each item's `body` is the
    /// `generalised_content` rewrite when present, else the raw `content`.
    pub async fn batch_share_items(
        &self,
        batch_id: &uuid::Uuid,
    ) -> Result<Option<(uuid::Uuid, String, Vec<ShareBatchItem>)>, String> {
        let head: Option<(uuid::Uuid, String)> = sqlx_core::query_as::query_as(
            "SELECT project_id, status::text FROM sensei.memory_share_batches WHERE id = $1",
        )
        .bind(batch_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let Some((project_id, status)) = head else {
            return Ok(None);
        };

        let rows: Vec<(uuid::Uuid, String, String, String)> = sqlx_core::query_as::query_as(
            // `generalised_content` ONLY — there is deliberately NO fallback to
            // `m.content`. The raw memory is the local reference: it may quote
            // real code, paths and decisions, and it is never a candidate for
            // sending. A COALESCE here meant "share the generalised version, or
            // the RAW one if nobody generalised it yet", which made the safe
            // path the one that happened to have run rather than the one that
            // was chosen.
            //
            // An empty body is returned rather than the row being dropped, so
            // the publish can report `held_not_generalised` and the user learns
            // WHY nothing was sent. A vanished row is indistinguishable from an
            // empty batch.
            "SELECT m.id, m.title,
                    COALESCE(btrim(m.generalised_content), ''),
                    m.type::text
               FROM sensei.memory_share_batch_members mm
               JOIN sensei.memories m ON m.id = mm.memory_id
              WHERE mm.batch_id = $1
              ORDER BY m.title",
        )
        .bind(batch_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let items = rows
            .into_iter()
            .map(|(memory_id, title, body, memory_type)| ShareBatchItem {
                memory_id,
                title,
                body,
                memory_type,
            })
            .collect();
        Ok(Some((project_id, status, items)))
    }

    /// The membership a project is bound to (`sensei.projects.dojo_id`), or `None`
    /// when the project is unbound / unknown. The routing anchor for C6.
    pub async fn project_bound_membership(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Option<uuid::Uuid>, String> {
        let row: Option<(Option<uuid::Uuid>,)> =
            sqlx_core::query_as::query_as("SELECT dojo_id FROM sensei.projects WHERE id = $1")
                .bind(project_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
        Ok(row.and_then(|(d,)| d))
    }

    /// The oldest `approved` share batch that still has at least one member memory
    /// with no `sent` outbox row — i.e. work the daemon still owes a Dōjō. Powers
    /// `GET /api/share-review/next-batch`. Returns `(batch_id, project_id,
    /// decided_at)`.
    pub async fn next_unsent_approved_batch(
        &self,
    ) -> Result<Option<(uuid::Uuid, uuid::Uuid, Option<String>)>, String> {
        let row: Option<(uuid::Uuid, uuid::Uuid, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT b.id, b.project_id, b.decided_at::text
               FROM sensei.memory_share_batches b
              WHERE b.status = 'approved'
                AND EXISTS (
                  SELECT 1 FROM sensei.memory_share_batch_members mm
                   WHERE mm.batch_id = b.id
                     AND NOT EXISTS (
                       SELECT 1 FROM sensei.dojo_outbox o
                        WHERE o.memory_id = mm.memory_id AND o.state = 'sent'))
              ORDER BY b.decided_at ASC NULLS LAST
              LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row)
    }

    /// The stable local contributor key (a machine-local secret that NEVER leaves
    /// the machine — only its rotated hash does, via [`crate::collective::anonymize`]).
    /// Get-or-create in `sensei.config` under `collective.contributor_key`.
    pub async fn get_or_create_contributor_key(&self) -> Result<String, String> {
        const KEY: &str = "collective.contributor_key";
        if let Some(v) = self.get_config(KEY).await?
            && !v.trim().is_empty()
        {
            return Ok(v);
        }
        let fresh = uuid::Uuid::new_v4().to_string();
        self.set_config(KEY, &fresh).await?;
        Ok(fresh)
    }

    /// Has this artifact `signature` already been published to `membership_id`?
    /// The pre-send dedup check — a retry after a federation drop skips a row
    /// already `sent` rather than double-publishing.
    pub async fn outbox_already_sent(
        &self,
        membership_id: &uuid::Uuid,
        signature: &str,
    ) -> Result<bool, String> {
        let row: Option<(bool,)> = sqlx_core::query_as::query_as(
            "SELECT state = 'sent' FROM sensei.dojo_outbox WHERE membership_id = $1 AND signature = $2")
            .bind(membership_id).bind(signature).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(b,)| b).unwrap_or(false))
    }

    /// Record a successful publish (idempotent on the `(membership_id, signature)`
    /// dedup key — a repeat send just refreshes the assigned seq/id).
    pub async fn outbox_mark_sent(
        &self,
        membership_id: &uuid::Uuid,
        batch_id: Option<&uuid::Uuid>,
        memory_id: Option<&uuid::Uuid>,
        signature: &str,
        sent_seq: i64,
        remote_id: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.dojo_outbox
                (membership_id, batch_id, memory_id, signature, state, sent_seq, remote_id, last_attempt_at)
             VALUES ($1,$2,$3,$4,'sent',$5,$6,now())
             ON CONFLICT (membership_id, signature) DO UPDATE SET
               state = 'sent', batch_id = EXCLUDED.batch_id, memory_id = EXCLUDED.memory_id,
               sent_seq = EXCLUDED.sent_seq, remote_id = EXCLUDED.remote_id,
               last_attempt_at = now(), updated_at = now()")
            .bind(membership_id).bind(batch_id).bind(memory_id).bind(signature).bind(sent_seq).bind(remote_id)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Record a non-sent outbox state (`held` | `queued` | `error`). Never
    /// downgrades an already-`sent` row (the `WHERE` guard), so a late held/queued
    /// signal can't erase a successful publish.
    pub async fn outbox_mark_state(
        &self,
        membership_id: &uuid::Uuid,
        batch_id: Option<&uuid::Uuid>,
        memory_id: Option<&uuid::Uuid>,
        signature: &str,
        state: &str,
    ) -> Result<(), String> {
        if !matches!(state, "held" | "queued" | "error" | "pending") {
            return Err(format!("invalid outbox state {state}"));
        }
        sqlx_core::query::query(
            "INSERT INTO sensei.dojo_outbox
                (membership_id, batch_id, memory_id, signature, state, last_attempt_at)
             VALUES ($1,$2,$3,$4,$5,now())
             ON CONFLICT (membership_id, signature) DO UPDATE SET
               state = EXCLUDED.state, batch_id = EXCLUDED.batch_id, memory_id = EXCLUDED.memory_id,
               last_attempt_at = now(), updated_at = now()
             WHERE sensei.dojo_outbox.state <> 'sent'",
        )
        .bind(membership_id)
        .bind(batch_id)
        .bind(memory_id)
        .bind(signature)
        .bind(state)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Dōjō downstream inbox (C7) — the DOWNSTREAM twin of the outbox above ──

    /// Mirror one pulled artifact into `sensei.dojo_inbox` as `pending`, deduped
    /// by `(membership_id, artifact_signature)`. Returns `true` when a NEW row was
    /// inserted, `false` when the artifact was already present in any state — so a
    /// re-pull is idempotent. scope/attribution ride as JSON text cast to jsonb
    /// (no sqlx json feature needed on the bind side).
    pub async fn upsert_dojo_inbox(
        &self,
        row: &crate::collective::inbox::InboxRow,
    ) -> Result<bool, String> {
        let scope = serde_json::to_string(&row.scope).map_err(|e| e.to_string())?;
        let attribution = serde_json::to_string(&row.attribution).map_err(|e| e.to_string())?;
        let inserted: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.dojo_inbox
                (membership_id, artifact_seq, artifact_signature, remote_id, kind, title, body, scope, attribution)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8::jsonb,$9::jsonb)
             ON CONFLICT (membership_id, artifact_signature) DO NOTHING
             RETURNING id")
            .bind(row.membership_id).bind(row.artifact_seq).bind(&row.signature).bind(&row.remote_id)
            .bind(&row.kind).bind(&row.title).bind(&row.body).bind(&scope).bind(&attribution)
            .fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(inserted.is_some())
    }

    /// Advance a membership's downstream pull cursor
    /// (`sensei.dojo_memberships.last_seq`).
    pub async fn set_dojo_pull_cursor(
        &self,
        membership_id: uuid::Uuid,
        cursor: i64,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.dojo_memberships SET last_seq = $2, updated_at = now() WHERE id = $1",
        )
        .bind(membership_id)
        .bind(cursor)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Load one inbox item by id.
    pub async fn get_dojo_inbox(
        &self,
        inbox_id: uuid::Uuid,
    ) -> Result<Option<crate::collective::inbox::InboxItem>, String> {
        let row: Option<(
            uuid::Uuid,
            uuid::Uuid,
            String,
            String,
            String,
            serde_json::Value,
            serde_json::Value,
            String,
            Option<String>,
            Option<uuid::Uuid>,
            Option<String>,
            String,
        )> = sqlx_core::query_as::query_as(&format!("{} WHERE id = $1", Self::DOJO_INBOX_SELECT))
            .bind(inbox_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(row.map(Self::map_dojo_inbox_row))
    }

    /// The daemon's downstream inbox across all memberships, ordered for the
    /// Upgrades list (pinned first, then newest; muted hidden unless
    /// `include_muted`). Reuses [`crate::collective::inbox::order_and_filter`] so
    /// the ordering contract has a single home.
    pub async fn list_dojo_inbox(
        &self,
        include_muted: bool,
    ) -> Result<Vec<crate::collective::inbox::InboxItem>, String> {
        let rows: Vec<(
            uuid::Uuid,
            uuid::Uuid,
            String,
            String,
            String,
            serde_json::Value,
            serde_json::Value,
            String,
            Option<String>,
            Option<uuid::Uuid>,
            Option<String>,
            String,
        )> = sqlx_core::query_as::query_as(&format!(
            "{} ORDER BY received_at DESC",
            Self::DOJO_INBOX_SELECT
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let items: Vec<_> = rows.into_iter().map(Self::map_dojo_inbox_row).collect();
        Ok(crate::collective::inbox::order_and_filter(items, include_muted))
    }

    /// Land an Applied principle/pattern: insert the memory (reusing
    /// [`Self::insert_memory`] — the shared memory-insert, never reimplemented)
    /// and flip the inbox row to `applied` + `applied_memory_id`. On a failed
    /// mark, the just-inserted memory is compensatingly deleted so a retry cannot
    /// double-land (the two writes are not one transaction because the insert goes
    /// through the shared helper; the compensating delete preserves idempotency).
    pub async fn land_dojo_inbox_memory(
        &self,
        inbox_id: uuid::Uuid,
        m: &InsertMemory,
    ) -> Result<uuid::Uuid, String> {
        let memory_id = self.insert_memory(m).await?;
        if let Err(e) = self.mark_dojo_inbox_applied(inbox_id, memory_id).await {
            if let Err(de) = sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1")
                .bind(memory_id)
                .execute(&self.pool)
                .await
            {
                tracing::error!(error = %de, memory = %memory_id, "dojo inbox: compensating memory delete failed after mark-applied error");
            }
            return Err(e);
        }
        Ok(memory_id)
    }

    /// Record why an Apply did not land (deferred kind / scope mismatch). The item
    /// stays `pending`.
    pub async fn set_dojo_inbox_note(
        &self,
        inbox_id: uuid::Uuid,
        note: String,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.dojo_inbox SET note = $2, updated_at = now() WHERE id = $1",
        )
        .bind(inbox_id)
        .bind(&note)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Set an inbox item's state (mute → `muted`, pin → `pinned`). Returns `false`
    /// when the id is unknown (drives a 404). Never lands anything.
    pub async fn set_dojo_inbox_state(
        &self,
        inbox_id: uuid::Uuid,
        state: &str,
    ) -> Result<bool, String> {
        if !matches!(state, "pending" | "applied" | "muted" | "pinned") {
            return Err(format!("invalid dojo_inbox state {state}"));
        }
        let res = sqlx_core::query::query(
            "UPDATE sensei.dojo_inbox SET state = $2, updated_at = now() WHERE id = $1",
        )
        .bind(inbox_id)
        .bind(state)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn create_knowledge_source(
        &self,
        s: &NewKnowledgeSource,
    ) -> Result<uuid::Uuid, String> {
        let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.knowledge_sources(kind, name, url, namespace_id, credential_ref, direction)
             VALUES($1,$2,$3,$4,$5,$6) RETURNING id")
            .bind(&s.kind).bind(&s.name).bind(&s.url).bind(s.namespace_id).bind(&s.credential_ref).bind(&s.direction)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(id)
    }

    pub async fn list_knowledge_sources(&self) -> Result<Vec<KnowledgeSource>, String> {
        let rows: Vec<(
            uuid::Uuid,
            String,
            String,
            String,
            Option<uuid::Uuid>,
            String,
            String,
            i64,
            bool,
        )> = sqlx_core::query_as::query_as(
            "SELECT id, kind, name, url, namespace_id, credential_ref, direction, last_seq, enabled
                   FROM sensei.knowledge_sources ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    kind,
                    name,
                    url,
                    namespace_id,
                    credential_ref,
                    direction,
                    last_seq,
                    enabled,
                )| KnowledgeSource {
                    id,
                    kind,
                    name,
                    url,
                    namespace_id,
                    credential_ref,
                    direction,
                    last_seq,
                    enabled,
                },
            )
            .collect())
    }

    pub async fn get_knowledge_source(
        &self,
        id: &uuid::Uuid,
    ) -> Result<Option<KnowledgeSource>, String> {
        let row: Option<(
            uuid::Uuid,
            String,
            String,
            String,
            Option<uuid::Uuid>,
            String,
            String,
            i64,
            bool,
        )> = sqlx_core::query_as::query_as(
            "SELECT id, kind, name, url, namespace_id, credential_ref, direction, last_seq, enabled
                   FROM sensei.knowledge_sources WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(
            |(id, kind, name, url, namespace_id, credential_ref, direction, last_seq, enabled)| {
                KnowledgeSource {
                    id,
                    kind,
                    name,
                    url,
                    namespace_id,
                    credential_ref,
                    direction,
                    last_seq,
                    enabled,
                }
            },
        ))
    }

    pub async fn set_source_cursor(&self, id: &uuid::Uuid, last_seq: i64) -> Result<(), String> {
        sqlx_core::query::query("UPDATE sensei.knowledge_sources SET last_seq = $2 WHERE id = $1")
            .bind(id)
            .bind(last_seq)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn delete_knowledge_source(&self, id: &uuid::Uuid) -> Result<bool, String> {
        let res = sqlx_core::query::query("DELETE FROM sensei.knowledge_sources WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }

    // ── Dōjō connections (daemon-side membership mirror) ───────────────
    //
    // Local mirror of the Dōjōs this install is connected to (Fork 1: the
    // authoritative dojo.memberships row lives in the Dōjō service DB). Mirrors
    // the knowledge_sources CRUD discipline; the credential lives in the OS
    // Keychain (credential_ref), never in these rows.

    /// Insert a Dōjō connection with the service-assigned `id` as the PK.
    pub async fn create_dojo_membership(
        &self,
        m: &NewDojoMembership,
    ) -> Result<uuid::Uuid, String> {
        let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.dojo_memberships
                (id, registry_url, tenant_key, dojo_url, kind, org_slugs, role,
                 authenticated_via, attribution_default, credential_ref, sync_status)
             VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING id",
        )
        .bind(m.id)
        .bind(&m.registry_url)
        .bind(&m.tenant_key)
        .bind(&m.dojo_url)
        .bind(&m.kind)
        .bind(&m.org_slugs)
        .bind(&m.role)
        .bind(&m.authenticated_via)
        .bind(&m.attribution_default)
        .bind(&m.credential_ref)
        .bind(&m.sync_status)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(id)
    }

    pub async fn list_dojo_memberships(&self) -> Result<Vec<DojoMembership>, String> {
        let rows: Vec<(
            uuid::Uuid,
            String,
            String,
            String,
            String,
            Vec<String>,
            String,
            String,
            String,
            String,
            String,
            i64,
            Option<String>,
            bool,
        )> = sqlx_core::query_as::query_as(&format!("{} ORDER BY created_at", Self::DOJO_SELECT))
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(Self::map_dojo_row).collect())
    }

    pub async fn get_dojo_membership(
        &self,
        id: &uuid::Uuid,
    ) -> Result<Option<DojoMembership>, String> {
        let row: Option<(
            uuid::Uuid,
            String,
            String,
            String,
            String,
            Vec<String>,
            String,
            String,
            String,
            String,
            String,
            i64,
            Option<String>,
            bool,
        )> = sqlx_core::query_as::query_as(&format!("{} WHERE id = $1", Self::DOJO_SELECT))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(row.map(Self::map_dojo_row))
    }

    /// Replace a membership's `org_slugs` (the git-remote owners it covers) —
    /// the org-tagging edit. Slugs are stored as given; callers normalise
    /// (lowercase/trim/dedup) upstream. Returns `false` if the id is unknown.
    pub async fn set_dojo_membership_orgs(
        &self,
        id: &uuid::Uuid,
        org_slugs: &[String],
    ) -> Result<bool, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.dojo_memberships SET org_slugs = $2, updated_at = now() WHERE id = $1",
        )
        .bind(id)
        .bind(org_slugs)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }

    /// Update a connection's sync status. Returns `false` if unknown.
    pub async fn set_dojo_sync_status(
        &self,
        id: &uuid::Uuid,
        sync_status: &str,
    ) -> Result<bool, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.dojo_memberships SET sync_status = $2, updated_at = now() WHERE id = $1",
        )
        .bind(id)
        .bind(sync_status)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn delete_dojo_membership(&self, id: &uuid::Uuid) -> Result<bool, String> {
        let res = sqlx_core::query::query("DELETE FROM sensei.dojo_memberships WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }

    /// Bind (or, with `None`, unbind) a project to a Dōjō membership by setting
    /// `sensei.projects.dojo_id`. Returns `false` if the project is unknown.
    pub async fn bind_project_to_dojo(
        &self,
        project_id: &uuid::Uuid,
        membership_id: Option<&uuid::Uuid>,
    ) -> Result<bool, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.projects SET dojo_id = $2, modified_at = now() WHERE id = $1",
        )
        .bind(project_id)
        .bind(membership_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }

    /// Projects bound to a membership (`projects.dojo_id = id`) — the
    /// connections pane's "bound projects" strip.
    pub async fn projects_bound_to_dojo(
        &self,
        membership_id: &uuid::Uuid,
    ) -> Result<Vec<(uuid::Uuid, String)>, String> {
        let rows: Vec<(uuid::Uuid, String)> = sqlx_core::query_as::query_as(
            "SELECT id, name FROM sensei.projects WHERE dojo_id = $1 ORDER BY name",
        )
        .bind(membership_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    pub async fn namespace_is_shareable(&self, namespace_id: &uuid::Uuid) -> Result<bool, String> {
        let row: Option<(bool,)> = sqlx_core::query_as::query_as(
            "SELECT s.shareable FROM sensei.namespaces n JOIN sensei.scopes s ON s.key = n.scope_key
              WHERE n.id = $1")
            .bind(namespace_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(b,)| b).unwrap_or(false))
    }

    pub async fn upsert_federated_memory(
        &self,
        source_id: &uuid::Uuid,
        remote_rule_id: &uuid::Uuid,
        content_hash: &str,
        memory_id: Option<&uuid::Uuid>,
        remote_seq: i64,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.federated_memories(knowledge_source_id, remote_rule_id, content_hash, memory_id, remote_seq)
             VALUES($1,$2,$3,$4,$5)
             ON CONFLICT(knowledge_source_id, remote_rule_id) DO UPDATE SET
               content_hash = EXCLUDED.content_hash,
               memory_id = COALESCE(EXCLUDED.memory_id, sensei.federated_memories.memory_id),
               remote_seq = EXCLUDED.remote_seq, synced_at = now()")
            .bind(source_id).bind(remote_rule_id).bind(content_hash).bind(memory_id).bind(remote_seq)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn find_federated_memory(
        &self,
        source_id: &uuid::Uuid,
        remote_rule_id: &uuid::Uuid,
    ) -> Result<Option<FederatedLink>, String> {
        let row: Option<(Option<uuid::Uuid>, i64)> = sqlx_core::query_as::query_as(
            "SELECT memory_id, remote_seq FROM sensei.federated_memories
              WHERE knowledge_source_id = $1 AND remote_rule_id = $2",
        )
        .bind(source_id)
        .bind(remote_rule_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|(memory_id, remote_seq)| FederatedLink { memory_id, remote_seq }))
    }

    /// Retire a federated memory (tombstone pulled from upstream). Only archives
    /// federated-origin rows, so a locally-authored/promoted memory is never force-archived.
    pub async fn archive_federated_memory(&self, memory_id: &uuid::Uuid) -> Result<bool, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.memories SET status = 'archived'::sensei.memory_status
              WHERE id = $1 AND origin = 'federated'",
        )
        .bind(memory_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }

    /// Fields to build a PublishedRule for a memory + its namespace identity.
    /// None if the memory has no namespace (unscoped).
    pub async fn memory_push_payload(
        &self,
        memory_id: &uuid::Uuid,
    ) -> Result<Option<MemoryPushPayload>, String> {
        let row: Option<(
            String,
            String,
            Option<String>,
            String,
            String,
            String,
            String,
            String,
            String,
        )> = sqlx_core::query_as::query_as(
            "SELECT m.title, m.content, m.impact, m.enforcement::text, m.type::text, m.origin,
                    n.scope_key, n.slug, n.name
               FROM sensei.memories m JOIN sensei.namespaces n ON n.id = m.namespace_id
              WHERE m.id = $1",
        )
        .bind(memory_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(
            |(title, content, impact, enforcement, rule_type, origin, scope_key, slug, name)| {
                MemoryPushPayload {
                    title,
                    content,
                    impact,
                    enforcement,
                    rule_type,
                    origin,
                    scope_key,
                    slug,
                    name,
                }
            },
        ))
    }

    // ── Scoped query helpers (#60) ─────────────────────────────────────
}
