//! `HiveStore` — the data layer for the hive: publish/pull of shared rules,
//! plus member/api-key/audit management.

use dojo_protocol::{
    ArtifactPayload, ArtifactPullResponse, ArtifactScope, ArtifactStatus, Attribution,
    PublishArtifactResponse, PublishedArtifact, PulledArtifact,
};
use hive_protocol::{PublishResponse, PublishedRule, PullResponse, PulledRule};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx_postgres::PgPool;
use subtle::ConstantTimeEq;
use uuid::Uuid;

/// Fixed key for the txn-scoped advisory lock that serializes `seq` assignment
/// (publish + retract). With `nextval` → write → commit all held under this lock,
/// seq order equals commit order, so the monotonic pull cursor is gap-free: a
/// puller that has advanced past seq N has necessarily already seen every
/// committed row with seq < N (no row can commit out of seq order and be skipped).
const SHARED_RULES_SEQ_LOCK: i64 = 0x6869_7665_5f73_6571; // ascii "hive_seq"

/// Advisory-lock key that serializes `dojo.artifacts.seq` assignment, the
/// artifact analogue of `SHARED_RULES_SEQ_LOCK`. A distinct key so artifact
/// publishes and rule publishes never contend (they share one embedded PG).
///
/// `pub(crate)` so the collective/promote runner ([`crate::collective::promote`])
/// takes the SAME lock when it advances `seq` on publish — publish and promote
/// must share this lock so the monotonic pull cursor stays gap-free.
pub(crate) const ARTIFACTS_SEQ_LOCK: i64 = 0x646f_6a6f_5f73_6571; // ascii "dojo_seq"

/// The authenticated identity resolved from a presented API key.
#[derive(Debug, Clone)]
pub struct Caller {
    pub member_id: Uuid,
    pub name: String,
    pub role: String,
}

/// A freshly issued API key — the plaintext is returned exactly once.
#[derive(Debug, Clone)]
pub struct IssuedKey {
    pub key_id: String,
    pub plaintext: String,
}

/// sha256 hex of a key (only the hash is ever stored).
fn hash_key(key: &str) -> String {
    let mut h = Sha256::new();
    h.update(key.as_bytes());
    format!("{:x}", h.finalize())
}

/// A fresh random API key: 32 random bytes, hex-encoded (64 chars).
fn random_key() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Data access over the hive Postgres pool.
#[derive(Clone)]
pub struct HiveStore {
    pool: PgPool,
}

impl HiveStore {
    /// Build a store over an existing pool (cloned from `HiveDb::pool()`).
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Borrow the underlying pool — used by the collective/promote runner
    /// ([`crate::collective::promote`]) so its transactional writes run against
    /// the same pool and reuse the shared [`ARTIFACTS_SEQ_LOCK`] seq discipline.
    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Publish a rule: upsert its namespace, then insert-or-bump the shared rule.
    /// First publish → version 1; republish of the same (namespace, content_hash)
    /// bumps the version and advances `seq`.
    ///
    /// Runs in one transaction holding `SHARED_RULES_SEQ_LOCK` so `seq` assignment
    /// is serialized with commit (gap-free pull cursor). `published_at` is stamped
    /// server-side (`now()`) — the client value is never trusted, so attribution
    /// timestamps can't be backdated. `published_by` is set by the
    /// caller-authenticated handler (see `api::publish_rule`).
    pub async fn publish(&self, r: &PublishedRule) -> Result<PublishResponse, String> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;

        sqlx_core::query::query("SELECT pg_advisory_xact_lock($1)")
            .bind(SHARED_RULES_SEQ_LOCK)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

        let (namespace_id,): (Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.namespaces(scope_key, slug, name)
             VALUES($1, $2, $3)
             ON CONFLICT(scope_key, slug) DO UPDATE SET name = EXCLUDED.name
             RETURNING id",
        )
        .bind(&r.scope_key)
        .bind(&r.namespace_slug)
        .bind(&r.namespace_name)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        let (id, version, seq): (Uuid, i32, i64) = sqlx_core::query_as::query_as(
            "INSERT INTO hive.shared_rules
               (namespace_id, content_hash, rule_type, title, content, impact,
                enforcement, status, version, origin_repo, published_by, published_at,
                seq, updated_at)
             VALUES
               ($1, $2, $3, $4, $5, $6, $7::sensei.enforcement, 'active', 1, $8, $9, now(),
                nextval('hive.shared_rules_seq'), now())
             ON CONFLICT(namespace_id, content_hash) DO UPDATE SET
               rule_type = EXCLUDED.rule_type,
               title = EXCLUDED.title,
               content = EXCLUDED.content,
               impact = EXCLUDED.impact,
               enforcement = EXCLUDED.enforcement,
               origin_repo = EXCLUDED.origin_repo,
               published_by = EXCLUDED.published_by,
               published_at = now(),
               status = 'active',
               version = hive.shared_rules.version + 1,
               seq = nextval('hive.shared_rules_seq'),
               updated_at = now()
             RETURNING id, version, seq",
        )
        .bind(namespace_id)
        .bind(&r.content_hash)
        .bind(&r.rule_type)
        .bind(&r.title)
        .bind(&r.content)
        .bind(&r.impact)
        .bind(&r.enforcement)
        .bind(&r.origin_repo)
        .bind(&r.published_by)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;

        Ok(PublishResponse {
            id: id.to_string(),
            version,
            seq,
        })
    }

    /// Pull all rule deltas with `seq > since`, ordered by `seq`. Each row is a
    /// full snapshot plus hive identity (id/seq/status/version). The returned
    /// cursor is the max `seq` seen (or `since` if the page is empty), to persist
    /// for the next pull.
    pub async fn pull_since(&self, since: i64) -> Result<PullResponse, String> {
        #[allow(clippy::type_complexity)]
        let rows: Vec<(
            Uuid,           // id
            i64,            // seq
            String,         // status
            i32,            // version
            String,         // content_hash
            String,         // scope_key
            String,         // namespace_slug
            String,         // namespace_name
            String,         // rule_type
            String,         // title
            String,         // content
            Option<String>, // impact
            String,         // enforcement
            Option<String>, // origin_repo
            String,         // published_by
            String,         // published_at
        )> = sqlx_core::query_as::query_as(
            "SELECT r.id, r.seq, r.status, r.version, r.content_hash,
                    n.scope_key, n.slug, n.name,
                    r.rule_type, r.title, r.content, r.impact,
                    r.enforcement::text,
                    r.origin_repo, r.published_by,
                    to_char(r.published_at, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
             FROM hive.shared_rules r
             JOIN sensei.namespaces n ON n.id = r.namespace_id
             WHERE r.seq > $1
             ORDER BY r.seq",
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let mut cursor = since;
        let mut rules = Vec::with_capacity(rows.len());
        for (
            id,
            seq,
            status,
            version,
            content_hash,
            scope_key,
            namespace_slug,
            namespace_name,
            rule_type,
            title,
            content,
            impact,
            enforcement,
            origin_repo,
            published_by,
            published_at,
        ) in rows
        {
            if seq > cursor {
                cursor = seq;
            }
            rules.push(PulledRule {
                id: id.to_string(),
                seq,
                status,
                version,
                rule: PublishedRule {
                    content_hash,
                    scope_key,
                    namespace_slug,
                    namespace_name,
                    rule_type,
                    title,
                    content,
                    impact,
                    enforcement,
                    origin_repo,
                    published_by,
                    published_at,
                },
            });
        }
        Ok(PullResponse { rules, cursor })
    }

    /// Tombstone a rule, advancing its `seq` so it surfaces in the next pull.
    /// Returns `true` if a (non-already-tombstoned) row was updated.
    pub async fn retract(&self, id: &str) -> Result<bool, String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        // Same lock as publish: serialize seq assignment with commit (gap-free cursor).
        sqlx_core::query::query("SELECT pg_advisory_xact_lock($1)")
            .bind(SHARED_RULES_SEQ_LOCK)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
        let res = sqlx_core::query::query(
            "UPDATE hive.shared_rules
             SET status = 'tombstoned',
                 seq = nextval('hive.shared_rules_seq'),
                 updated_at = now()
             WHERE id = $1 AND status <> 'tombstoned'",
        )
        .bind(uuid)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }

    /// Create a federation member, returning its id.
    pub async fn create_member(
        &self,
        name: &str,
        email: Option<&str>,
        role: &str,
    ) -> Result<Uuid, String> {
        let (id,): (Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO hive.members(name, email, role) VALUES($1, $2, $3) RETURNING id",
        )
        .bind(name)
        .bind(email)
        .bind(role)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(id)
    }

    /// Issue an API key for a member. Stores only the sha256 hash; returns the
    /// plaintext (shown once) along with the key id.
    pub async fn issue_key(
        &self,
        member_id: &Uuid,
        label: Option<&str>,
    ) -> Result<IssuedKey, String> {
        let plaintext = random_key();
        let key_hash = hash_key(&plaintext);
        let (id,): (Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO hive.api_keys(member_id, key_hash, label) VALUES($1, $2, $3) RETURNING id",
        )
        .bind(member_id)
        .bind(&key_hash)
        .bind(label)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(IssuedKey {
            key_id: id.to_string(),
            plaintext,
        })
    }

    /// Resolve a presented key to its caller. Only active (non-revoked, member
    /// not disabled) keys match. The lookup filters by `key_hash` (indexed by
    /// `api_keys_key_hash_idx`) so it stays O(log n) as the key count grows,
    /// then constant-time-compares the single returned row before returning —
    /// defense in depth against any exotic timing at the pg side. On a hit,
    /// `last_used_at` is stamped.
    pub async fn find_member_by_key(&self, presented: &str) -> Result<Option<Caller>, String> {
        let presented_hash = hash_key(presented);
        let row: Option<(Uuid, Uuid, String, String, String)> = sqlx_core::query_as::query_as(
            "SELECT k.id, m.id, m.name, m.role, k.key_hash
             FROM hive.api_keys k
             JOIN hive.members m ON m.id = k.member_id
             WHERE k.key_hash = $1 AND k.revoked_at IS NULL AND m.disabled_at IS NULL
             LIMIT 1",
        )
        .bind(&presented_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let Some((key_id, member_id, name, role, key_hash)) = row else { return Ok(None) };
        let matches: bool = key_hash
            .as_bytes()
            .ct_eq(presented_hash.as_bytes())
            .into();
        if !matches {
            return Ok(None);
        }
        sqlx_core::query::query(
            "UPDATE hive.api_keys SET last_used_at = now() WHERE id = $1",
        )
        .bind(key_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(Some(Caller { member_id, name, role }))
    }

    /// Revoke an API key by id.
    pub async fn revoke_key(&self, key_id: &str) -> Result<bool, String> {
        let uuid = Uuid::parse_str(key_id).map_err(|e| e.to_string())?;
        let res =
            sqlx_core::query::query("UPDATE hive.api_keys SET revoked_at = now() WHERE id = $1")
                .bind(uuid)
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }

    /// Append an entry to the audit log.
    pub async fn record_audit(
        &self,
        member_id: Option<&Uuid>,
        action: &str,
        target: Option<&str>,
        detail: serde_json::Value,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO hive.audit_log(member_id, action, target, detail)
             VALUES($1, $2, $3, $4)",
        )
        .bind(member_id)
        .bind(action)
        .bind(target)
        .bind(detail)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Dōjō multi-tenancy (additive; tenant-scoped artifact federation) ──────

    /// Resolve a tenant discovery key (`"<origin>/<org>[/<dojo>]"`) to its
    /// tenant id. `None` when no such tenant exists (the caller maps that to
    /// 404). Every dojo.* query is scoped by the id this returns.
    pub async fn resolve_tenant(&self, key: &str) -> Result<Option<Uuid>, String> {
        let row: Option<(Uuid,)> =
            sqlx_core::query_as::query_as("SELECT id FROM dojo.tenants WHERE key = $1 LIMIT 1")
                .bind(key)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
        Ok(row.map(|(id,)| id))
    }

    /// Create a tenant Dōjō (the isolation boundary). Used to stand up a tenant
    /// before its artifacts flow (console/admin surface is C13). Returns its id.
    pub async fn create_tenant(
        &self,
        key: &str,
        origin: &str,
        org: &str,
        dojo: Option<&str>,
        name: &str,
        dojo_url: &str,
    ) -> Result<Uuid, String> {
        let (id,): (Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO dojo.tenants(key, origin, org, dojo, name, dojo_url)
             VALUES($1, $2::dojo.tenant_origin, $3, $4, $5, $6)
             RETURNING id",
        )
        .bind(key)
        .bind(origin)
        .bind(org)
        .bind(dojo)
        .bind(name)
        .bind(dojo_url)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(id)
    }

    /// Enrol a user in a tenant (the Supabase-JWT plane authenticates against
    /// this). `dojo_url` is inherited from the tenant. Returns the membership id.
    pub async fn create_membership(
        &self,
        tenant_id: &Uuid,
        user_id: &Uuid,
        kind: &str,
        authenticated_via: &str,
        role: &str,
    ) -> Result<Uuid, String> {
        let (id,): (Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO dojo.memberships(tenant_id, user_id, dojo_url, kind, authenticated_via, role)
             SELECT $1, $2, t.dojo_url, $3::dojo.membership_kind, $4::dojo.auth_method, $5::dojo.member_role
             FROM dojo.tenants t WHERE t.id = $1
             RETURNING id",
        )
        .bind(tenant_id)
        .bind(user_id)
        .bind(kind)
        .bind(authenticated_via)
        .bind(role)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(id)
    }

    /// Set a tenant's visibility scope (`private` | `global`). Used by the
    /// provisioning CLI to apply `--scope` after the tenant row exists (the DDL
    /// default is `private`). Idempotent — re-setting the same scope is a no-op
    /// update. Returns whether a row was updated.
    pub async fn set_tenant_scope(&self, tenant_id: &Uuid, scope: &str) -> Result<bool, String> {
        let res = sqlx_core::query::query(
            "UPDATE dojo.tenants SET scope = $2::dojo.tenant_scope, updated_at = now()
             WHERE id = $1",
        )
        .bind(tenant_id)
        .bind(scope)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }

    /// Resolve a verified JWT subject to its role in `tenant_id`. `None` when the
    /// subject isn't a member here (or the subject isn't a uuid). Only active
    /// (non-disabled) memberships match.
    pub async fn find_membership_role(
        &self,
        tenant_id: &Uuid,
        user_id: &str,
    ) -> Result<Option<String>, String> {
        // A JWT `sub` that isn't a uuid can't match `dojo.memberships.user_id`.
        let Ok(uid) = Uuid::parse_str(user_id) else {
            return Ok(None);
        };
        let row: Option<(String,)> = sqlx_core::query_as::query_as(
            "SELECT role::text FROM dojo.memberships
             WHERE tenant_id = $1 AND user_id = $2 AND disabled_at IS NULL
             LIMIT 1",
        )
        .bind(tenant_id)
        .bind(uid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|(r,)| r))
    }

    /// Publish (contribute) an artifact under `tenant_id`, assigning a fresh
    /// `seq` under `ARTIFACTS_SEQ_LOCK` so the pull cursor stays gap-free (the
    /// artifact analogue of [`publish`](Self::publish)). `contributed_by` is set
    /// from the authenticated caller — the body can't spoof it. The row lands at
    /// the DDL-default status (`submitted`, pending triage); triage/approval
    /// (→ `published`) is C8.
    pub async fn publish_artifact(
        &self,
        tenant_id: &Uuid,
        a: &PublishedArtifact,
        contributed_by: Option<Uuid>,
    ) -> Result<PublishArtifactResponse, String> {
        let payload = serde_json::to_value(&a.payload).map_err(|e| e.to_string())?;
        let scope = serde_json::to_value(&a.scope).map_err(|e| e.to_string())?;
        let attribution = serde_json::to_value(&a.attribution).map_err(|e| e.to_string())?;
        let engagement_id = match &a.engagement_id {
            Some(s) => Some(Uuid::parse_str(s).map_err(|e| e.to_string())?),
            None => None,
        };

        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        sqlx_core::query::query("SELECT pg_advisory_xact_lock($1)")
            .bind(ARTIFACTS_SEQ_LOCK)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

        let (id, seq): (Uuid, i64) = sqlx_core::query_as::query_as(
            "INSERT INTO dojo.artifacts
               (tenant_id, engagement_id, kind, title, body, payload, scope,
                signature, attribution, dereferenced, contributed_by, seq)
             VALUES
               ($1, $2, $3::dojo.artifact_kind, $4, $5, $6::jsonb, $7::jsonb,
                $8, $9::jsonb, $10, $11, nextval('dojo.artifacts_seq'))
             RETURNING id, seq",
        )
        .bind(tenant_id)
        .bind(engagement_id)
        .bind(a.kind.as_db_str())
        .bind(&a.title)
        .bind(&a.body)
        .bind(payload)
        .bind(scope)
        .bind(&a.signature)
        .bind(attribution)
        .bind(a.dereferenced)
        .bind(contributed_by)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(PublishArtifactResponse {
            id: id.to_string(),
            seq,
        })
    }

    /// Pull a tenant's artifacts with `seq > since`, ordered by `seq` (the
    /// artifact analogue of [`pull_since`](Self::pull_since)). Strictly scoped to
    /// `tenant_id`, so one tenant never sees another's artifacts. The returned
    /// cursor is the max `seq` seen (or `since` if empty).
    pub async fn pull_artifacts_since(
        &self,
        tenant_id: &Uuid,
        since: i64,
    ) -> Result<ArtifactPullResponse, String> {
        #[allow(clippy::type_complexity)]
        let rows: Vec<(
            Uuid,               // id
            i64,                // seq
            String,             // status
            String,             // tenant key
            Option<Uuid>,       // engagement_id
            String,             // kind
            String,             // title
            String,             // body
            serde_json::Value,  // payload
            serde_json::Value,  // scope
            String,             // signature
            serde_json::Value,  // attribution
            bool,               // dereferenced
            Option<Uuid>,       // contributed_by
            Option<String>,     // published_at
        )> = sqlx_core::query_as::query_as(
            "SELECT a.id, a.seq, a.status::text, t.key, a.engagement_id,
                    a.kind::text, a.title, a.body, a.payload, a.scope,
                    a.signature, a.attribution, a.dereferenced, a.contributed_by,
                    to_char(a.published_at, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
             FROM dojo.artifacts a
             JOIN dojo.tenants t ON t.id = a.tenant_id
             WHERE a.tenant_id = $1 AND a.seq > $2
             ORDER BY a.seq",
        )
        .bind(tenant_id)
        .bind(since)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let mut cursor = since;
        let mut artifacts = Vec::with_capacity(rows.len());
        for (
            id,
            seq,
            status,
            tenant_key,
            engagement_id,
            kind,
            title,
            body,
            payload,
            scope,
            signature,
            attribution,
            dereferenced,
            contributed_by,
            published_at,
        ) in rows
        {
            if seq > cursor {
                cursor = seq;
            }
            let kind = dojo_protocol::ArtifactKind::from_db_str(&kind)
                .ok_or_else(|| format!("unknown artifact kind '{kind}'"))?;
            let status = ArtifactStatus::from_db_str(&status)
                .ok_or_else(|| format!("unknown artifact status '{status}'"))?;
            let payload: ArtifactPayload =
                serde_json::from_value(payload).map_err(|e| e.to_string())?;
            let scope: ArtifactScope = serde_json::from_value(scope).map_err(|e| e.to_string())?;
            let attribution: Attribution =
                serde_json::from_value(attribution).map_err(|e| e.to_string())?;
            artifacts.push(PulledArtifact {
                id: id.to_string(),
                seq,
                status,
                artifact: PublishedArtifact {
                    signature,
                    tenant_key,
                    engagement_id: engagement_id.map(|u| u.to_string()),
                    kind,
                    title,
                    body,
                    payload,
                    scope,
                    attribution,
                    dereferenced,
                    contributed_by: contributed_by.map(|u| u.to_string()),
                    published_at,
                },
            });
        }
        Ok(ArtifactPullResponse { artifacts, cursor })
    }
}
