# Corrections Aggregation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Cluster recurring corrective developer prompts globally into project-tagged corrections (canonical text + LLM suggestion + linked memory), stored in a new `inference.corrections` table and exposed via `GET /api/corrections` + `GET /api/projects/{id}/corrections`.

**Architecture:** A dedicated global task (`AggregateCorrections`, enqueued once per analyzer-scheduler tick) pulls all user prompts, keeps corrections via the existing L1 classifiers (regex recall → `classify_batch` precision), embeds them through the 384-dim `embed` chain, greedily + deterministically clusters by cosine similarity (lexical fallback when no embed model), summarizes each surviving cluster with one gateway call (canonical text + suggestion + memory link, graceful), and upserts idempotently by a deterministic seed-derived signature (then prunes stale signatures). Pure logic (clustering, prompt build/parse, row assembly) is isolated in unit-tested modules; IO lives in the handler + `pg_store`.

**Tech Stack:** Rust (axum, sqlx via `sqlx_core`, tokio), the in-house `gateway` crate, PostgreSQL (`sensei` runtime DB / `sensei_test` test DB), `dbd` for DDL.

**Spec:** `docs/superpowers/specs/2026-06-24-corrections-aggregation-design.md`

---

## File Structure

**Create:**
- `database/ddl/table/inference/corrections.ddl` — new table (full DDL).
- `crates/senseid/src/corrections.rs` — pure clustering / normalize / cosine / signature / `Cluster` / `CorrectionRow` / `lexical_cluster`. No IO, no gateway. Unit-tested.
- `crates/senseid/src/tasks/handlers/corrections_llm.rs` — pure `build_user_message` / `parse_response` + graceful async `summarize_cluster`. Mirrors `prompt_classify.rs`.
- `crates/senseid/src/tasks/handlers/corrections.rs` — orchestrator `aggregate_corrections` + pure `build_rows` + private `embed_items`.
- `crates/senseid/src/api/handlers/corrections.rs` — two read handlers.

**Modify:**
- `crates/senseid/src/main.rs` — `pub mod corrections;`.
- `crates/senseid/src/tasks/mod.rs` — `TaskKind::AggregateCorrections` (variant + `Display` + watchdog bucket).
- `crates/senseid/src/tasks/handlers/mod.rs` — `mod corrections; mod corrections_llm;` + `pub use corrections::aggregate_corrections;`.
- `crates/senseid/src/tasks/executor.rs` — route the new kind.
- `crates/senseid/src/tasks/analyzer_scheduler.rs` — enqueue the global task once per tick when any project was due.
- `crates/senseid/src/db/pg_store.rs` — `get_all_user_prompts`, `get_learned_memories_for_matching`, `upsert_correction`, `delete_corrections_not_in`, `list_corrections`, `list_corrections_for_project` (+ private `query_corrections`).
- `crates/senseid/src/api/handlers/mod.rs` — `pub(crate) mod corrections;`.
- `crates/senseid/src/api/routes.rs` — import + two routes.

**Convention notes (read before coding):**
- `pg_store` binds `serde_json::Value` to `jsonb` columns fine (see `upsert_pattern`), but **decoding** a `json`/`jsonb` column is avoided — cast to `::text` and parse in Rust (see `get_patterns_for_generation`). The read path here follows that: `json_agg(...)::text`.
- `assistant_events.ts` is `bigint` epoch-ms; convert to `timestamptz` in Rust with `DateTime::<Utc>::from_timestamp_millis`.
- Gateway request shapes: embeds use `Capability::TextEmbed` + `Payload::Embed { texts }` + `chain: Some("embed")` → `resp.embeddings`; chat uses `Capability::TextChat` + `Payload::Chat {..}` + a named `chain` → `resp.content` / `resp.success` (see `embed.rs` and `prompt_classify.rs`).
- DB tests connect to `TEST_DATABASE_URL` or `sensei_test` via the `pg_store()` test helper; they require the table to exist there (Task 1 applies it).

---

## Task 1: DDL — `inference.corrections` table

**Files:**
- Create: `database/ddl/table/inference/corrections.ddl`

- [ ] **Step 1: Write the DDL file**

Create `database/ddl/table/inference/corrections.ddl`:

```sql
set search_path to inference, sensei, extensions;

create table if not exists corrections (
  id            uuid         primary key default gen_random_uuid()
, signature     text         not null unique
, text          text         not null
, suggestion    text
, count         integer      not null default 0
, project_ids   uuid[]       not null default '{}'
, last_seen     timestamptz
, memory_id     uuid         references sensei.memories(id) on delete set null
, instances     jsonb        not null default '[]'
, detected_at   timestamptz  not null default now()
, modified_at   timestamptz  not null default now()
);

create index if not exists corrections_project_ids_idx
    on corrections using gin(project_ids);

create index if not exists corrections_count_idx
    on corrections(count desc);

comment on table corrections is
'Recurring developer corrections, clustered globally across projects (analyzer #65 step 5).
One row per recurring correction cluster: similar corrective prompts grouped by
embedding (or lexical) similarity. Re-derived idempotently by the AggregateCorrections
task; `signature` is the stable natural key.';

comment on column corrections.id           is 'Surrogate primary key (UUID). Stable across runs via upsert-on-signature.';
comment on column corrections.signature    is 'Deterministic cluster identity: hash(seed_session + normalized seed prompt). Stable as the cluster grows.';
comment on column corrections.text         is 'Canonical correction statement (LLM). Falls back to the seed member''s normalized snippet when no chat model is available.';
comment on column corrections.suggestion   is 'LLM advisory on what to do (reinforce a memory / add a rule / write a skill). Null when no chat model.';
comment on column corrections.count        is 'Number of corrective prompts in the cluster.';
comment on column corrections.project_ids  is 'Distinct projects the correction appeared in. Names resolved by the API.';
comment on column corrections.last_seen    is 'Most recent corrective prompt in the cluster.';
comment on column corrections.memory_id    is 'Related learned memory (LLM-matched from a shortlist), or null.';
comment on column corrections.instances    is 'Provenance: [{project_id, session_id, ts, prompt}] — the member corrective prompts (snippet).';
comment on column corrections.detected_at  is 'When this cluster was first derived.';
comment on column corrections.modified_at  is 'When this row was last upserted.';
```

- [ ] **Step 2: Apply to the runtime DB (`sensei`)**

From the repo root:

Run: `cd database && dbd deploy`
Expected: deploy succeeds; the new `inference.corrections` table is created (it's `create table if not exists`, so it's additive).

If `dbd` is not configured for direct apply in this environment, apply the file directly (idempotent):
Run: `psql -p 7744 -d sensei -f database/ddl/table/inference/corrections.ddl`
Expected: `CREATE TABLE` / `CREATE INDEX` / `COMMENT` lines, no errors.

- [ ] **Step 3: Apply to the test DB (`sensei_test`)**

Run: `psql -d "${TEST_DATABASE_URL:-sensei_test}" -f database/ddl/table/inference/corrections.ddl`
Expected: `CREATE TABLE` / `CREATE INDEX`, no errors.

- [ ] **Step 4: Verify the table exists**

Run: `psql -p 7744 -d sensei -c '\d inference.corrections'`
Expected: lists columns `id, signature, text, suggestion, count, project_ids, last_seen, memory_id, instances, detected_at, modified_at` and the unique index on `signature` + the GIN index on `project_ids`.

- [ ] **Step 5: Commit**

```bash
git add database/ddl/table/inference/corrections.ddl
git commit -m "feat(db): add inference.corrections table for corrections aggregation"
```

---

## Task 2: Pure clustering module `corrections.rs`

**Files:**
- Create: `crates/senseid/src/corrections.rs`
- Modify: `crates/senseid/src/main.rs` (add `pub mod corrections;`)
- Test: inline `#[cfg(test)] mod tests` in `corrections.rs`

- [ ] **Step 1: Register the module**

In `crates/senseid/src/main.rs`, add after `pub mod pattern_effectiveness;` (line ~22):

```rust
pub mod corrections;
```

- [ ] **Step 2: Write the module with failing tests**

Create `crates/senseid/src/corrections.rs`:

```rust
//! Corrections aggregation (#65 step 5) — pure clustering of recurring corrective
//! prompts into project-tagged groups. No IO and no gateway here: the handler
//! (`tasks/handlers/corrections.rs`) supplies embeddings and persistence so this
//! logic is unit-testable over plain data.

/// Cosine-similarity threshold: two corrective prompts join the same cluster when
/// their embeddings are at least this similar. Tunable.
pub const SIMILARITY_THRESHOLD: f32 = 0.82;

/// A cluster must reach this many members to surface as a "recurring" correction.
pub const CORRECTION_CLUSTER_MIN: usize = 2;

/// Max chars of the normalized snippet / canonical-text fallback.
const SNIPPET_MAX: usize = 200;

/// One corrective prompt projected to the fields clustering needs.
#[derive(Debug, Clone)]
pub struct CorrItem {
    pub project_id: uuid::Uuid,
    pub session_id: String,
    pub ts: i64, // epoch ms (activity.assistant_events.ts)
    pub prompt: String,
}

/// A derived cluster of similar corrective prompts.
#[derive(Debug, Clone, PartialEq)]
pub struct Cluster {
    pub signature: String,
    pub representative_text: String, // normalized snippet of the seed (earliest) member
    pub count: usize,
    pub last_seen_ms: i64, // max ts across members
    pub project_ids: Vec<uuid::Uuid>, // distinct, sorted
    pub member_idxs: Vec<usize>, // indices into the input slice
}

/// A row ready to upsert into `inference.corrections`. Plain data carrier shared
/// between the handler (which fills text/suggestion/memory from the LLM) and
/// `pg_store::upsert_correction`.
#[derive(Debug, Clone)]
pub struct CorrectionRow {
    pub signature: String,
    pub text: String,
    pub suggestion: Option<String>,
    pub count: i32,
    pub project_ids: Vec<uuid::Uuid>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub memory_id: Option<uuid::Uuid>,
    pub instances: serde_json::Value,
}

/// Normalize a prompt for lexical comparison + signature stability: lowercase,
/// collapse runs of whitespace/punctuation to single spaces, drop other symbols,
/// cap length on a char boundary.
pub fn normalize(prompt: &str) -> String {
    let lowered = prompt.trim().to_lowercase();
    let mut s = String::with_capacity(lowered.len());
    let mut prev_space = false;
    for ch in lowered.chars() {
        if ch.is_alphanumeric() {
            s.push(ch);
            prev_space = false;
        } else if (ch.is_whitespace() || ch.is_ascii_punctuation()) && !prev_space && !s.is_empty() {
            s.push(' ');
            prev_space = true;
        }
    }
    let trimmed = s.trim_end();
    trimmed.chars().take(SNIPPET_MAX).collect()
}

/// Cosine similarity of two equal-length vectors. Returns 0.0 for mismatched or
/// zero-norm inputs (can't compare).
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

/// Deterministic signature from a cluster's seed (its earliest-by-ts member).
/// FNV-1a over "session:normalized_prompt" → hex. Independent of LLM output, so
/// re-runs upsert the same row whether or not models ran.
pub fn signature(seed_session: &str, seed_prompt: &str) -> String {
    let key = format!("{}:{}", seed_session, normalize(seed_prompt));
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in key.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("corr-{:016x}", hash)
}

/// Internal accumulator while building a cluster.
struct ClusterAcc {
    seed_idx: usize,
    members: Vec<usize>,
    last_seen_ms: i64,
    projects: Vec<uuid::Uuid>,
}

impl ClusterAcc {
    fn new(seed_idx: usize, item: &CorrItem) -> Self {
        Self {
            seed_idx,
            members: vec![seed_idx],
            last_seen_ms: item.ts,
            projects: vec![item.project_id],
        }
    }
    fn push(&mut self, idx: usize, item: &CorrItem) {
        self.members.push(idx);
        if item.ts > self.last_seen_ms {
            self.last_seen_ms = item.ts;
        }
        if !self.projects.contains(&item.project_id) {
            self.projects.push(item.project_id);
        }
    }
    fn finish(mut self, items: &[CorrItem]) -> Cluster {
        let seed = &items[self.seed_idx];
        self.projects.sort();
        Cluster {
            signature: signature(&seed.session_id, &seed.prompt),
            representative_text: normalize(&seed.prompt),
            count: self.members.len(),
            last_seen_ms: self.last_seen_ms,
            project_ids: self.projects,
            member_idxs: self.members,
        }
    }
}

/// Greedy, deterministic clustering. `items` and `embeddings` are parallel and
/// `items` MUST be sorted by `ts` ascending so the earliest member seeds each
/// cluster. Each item joins the first existing cluster whose **seed** embedding is
/// within `threshold`, else it seeds a new cluster.
pub fn cluster(items: &[CorrItem], embeddings: &[Vec<f32>], threshold: f32) -> Vec<Cluster> {
    let mut accs: Vec<ClusterAcc> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let mut joined = false;
        if let Some(emb) = embeddings.get(i) {
            for acc in accs.iter_mut() {
                if let Some(seed_emb) = embeddings.get(acc.seed_idx) {
                    if cosine(emb, seed_emb) >= threshold {
                        acc.push(i, item);
                        joined = true;
                        break;
                    }
                }
            }
        }
        if !joined {
            accs.push(ClusterAcc::new(i, item));
        }
    }
    accs.into_iter().map(|a| a.finish(items)).collect()
}

/// Lexical fallback when embeddings are unavailable: group by normalized text.
/// Deterministic. `items` must be sorted by `ts` ascending.
pub fn lexical_cluster(items: &[CorrItem]) -> Vec<Cluster> {
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, item) in items.iter().enumerate() {
        groups.entry(normalize(&item.prompt)).or_default().push(i);
    }
    groups
        .into_values()
        .map(|idxs| {
            let mut acc = ClusterAcc::new(idxs[0], &items[idxs[0]]);
            for &i in &idxs[1..] {
                acc.push(i, &items[i]);
            }
            acc.finish(items)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(pid: uuid::Uuid, session: &str, ts: i64, prompt: &str) -> CorrItem {
        CorrItem { project_id: pid, session_id: session.into(), ts, prompt: prompt.into() }
    }

    #[test]
    fn normalize_lowercases_strips_punct_collapses_ws() {
        assert_eq!(normalize("  Use `$state(…)`, please!! "), "use state please");
        // idempotent
        let once = normalize("No, that's WRONG -- revert it.");
        assert_eq!(normalize(&once), once);
    }

    #[test]
    fn cosine_identity_orthogonal_and_mismatch() {
        assert!((cosine(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        assert!(cosine(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-6);
        assert_eq!(cosine(&[1.0, 0.0], &[1.0]), 0.0, "mismatched length");
        assert_eq!(cosine(&[0.0, 0.0], &[1.0, 1.0]), 0.0, "zero norm");
    }

    #[test]
    fn signature_is_deterministic_and_text_sensitive() {
        let a = signature("s1", "No, revert that change");
        assert_eq!(a, signature("s1", "no,  revert   that change"), "stable under normalization");
        assert_ne!(a, signature("s2", "No, revert that change"), "session-sensitive");
        assert_ne!(a, signature("s1", "use $state instead"), "text-sensitive");
    }

    #[test]
    fn cluster_merges_similar_and_separates_distinct() {
        let p = uuid::Uuid::new_v4();
        let items = vec![
            item(p, "s1", 100, "use $state"),
            item(p, "s2", 200, "use $state please"),
            item(p, "s3", 300, "add a retry"),
        ];
        // embeddings: first two near-identical, third orthogonal
        let embs = vec![vec![1.0, 0.0], vec![0.99, 0.14], vec![0.0, 1.0]];
        let mut clusters = cluster(&items, &embs, 0.82);
        clusters.sort_by_key(|c| c.count);
        assert_eq!(clusters.len(), 2);
        let big = clusters.iter().find(|c| c.count == 2).unwrap();
        assert_eq!(big.last_seen_ms, 200, "max ts of members");
        assert_eq!(big.project_ids, vec![p]);
    }

    #[test]
    fn cluster_seed_is_earliest_and_signature_stable_as_it_grows() {
        let p = uuid::Uuid::new_v4();
        let base = vec![item(p, "s1", 100, "use $state"), item(p, "s2", 200, "use $state please")];
        let embs2 = vec![vec![1.0, 0.0], vec![0.99, 0.14]];
        let c2 = cluster(&base, &embs2, 0.82);
        assert_eq!(c2.len(), 1);
        let sig2 = c2[0].signature.clone();

        // A later similar member joins; seed (earliest ts=100) unchanged → same signature.
        let grown = vec![
            item(p, "s1", 100, "use $state"),
            item(p, "s2", 200, "use $state please"),
            item(p, "s3", 300, "please use $state here"),
        ];
        let embs3 = vec![vec![1.0, 0.0], vec![0.99, 0.14], vec![0.98, 0.2]];
        let c3 = cluster(&grown, &embs3, 0.82);
        assert_eq!(c3.len(), 1);
        assert_eq!(c3[0].count, 3);
        assert_eq!(c3[0].signature, sig2, "signature stable as the cluster grows");
    }

    #[test]
    fn cluster_dedups_projects() {
        let p1 = uuid::Uuid::new_v4();
        let p2 = uuid::Uuid::new_v4();
        let items = vec![
            item(p1, "s1", 100, "use $state"),
            item(p2, "s2", 200, "use $state please"),
            item(p1, "s3", 300, "use $state now"),
        ];
        let embs = vec![vec![1.0, 0.0], vec![0.99, 0.14], vec![0.98, 0.2]];
        let c = cluster(&items, &embs, 0.82);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].project_ids.len(), 2, "two distinct projects, deduped");
    }

    #[test]
    fn lexical_cluster_groups_by_normalized_text() {
        let p = uuid::Uuid::new_v4();
        let items = vec![
            item(p, "s1", 100, "Revert that!"),
            item(p, "s2", 200, "revert  that"),
            item(p, "s3", 300, "add a retry"),
        ];
        let mut c = lexical_cluster(&items);
        c.sort_by_key(|x| x.count);
        assert_eq!(c.len(), 2);
        assert_eq!(c.last().unwrap().count, 2);
    }
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p senseid --lib corrections::tests`
Expected: PASS (all 7 tests).

- [ ] **Step 4: Commit**

```bash
git add crates/senseid/src/corrections.rs crates/senseid/src/main.rs
git commit -m "feat: pure corrections clustering module (normalize, cosine, signature, cluster)"
```

---

## Task 3: Pure LLM module `corrections_llm.rs`

**Files:**
- Create: `crates/senseid/src/tasks/handlers/corrections_llm.rs`
- Modify: `crates/senseid/src/tasks/handlers/mod.rs` (add `mod corrections_llm;`)
- Test: inline `#[cfg(test)] mod tests`

- [ ] **Step 1: Declare the module**

In `crates/senseid/src/tasks/handlers/mod.rs`, add next to `mod prompt_classify;`:

```rust
mod corrections_llm;
```

- [ ] **Step 2: Write the module with failing tests**

Create `crates/senseid/src/tasks/handlers/corrections_llm.rs`:

```rust
//! Per-cluster canonicalization (#65 step 5): one gateway call turns a cluster of
//! similar corrective prompts into a clean rule statement + an advisory suggestion
//! + an optional link to an existing memory. Mirrors `prompt_classify`: pure
//! build/parse + a graceful async call that degrades to `None` (caller then falls
//! back to the cluster's representative snippet).

use gateway::Gateway;

#[derive(Debug, Clone, PartialEq)]
pub struct ClusterSummary {
    pub text: String,
    pub suggestion: Option<String>,
    pub memory_id: Option<uuid::Uuid>,
}

const SYSTEM: &str = "You distill a cluster of a developer's repeated corrections to an AI coding agent into one durable rule. \
Given the example prompts and an optional list of existing memories, reply with ONLY a JSON object: \
{\"text\": <one-sentence canonical correction phrased as an imperative rule>, \
\"suggestion\": <one sentence on what to do about it, e.g. reinforce a memory, add a rule, or write a skill>, \
\"memory_id\": <the id of the most related memory from the list, or null>}. \
No prose, no code fences.";

/// Token budget for the summary (one short JSON object; reasoning headroom).
const MAX_TOKENS: u32 = 512;
/// Max representative prompts shown to the model per cluster.
const MAX_REPS: usize = 5;

/// Build the user message: the representative prompts + the candidate memories to
/// match against. Bounded per item to keep the request small.
pub fn build_user_message(reps: &[&str], memories: &[(uuid::Uuid, String)]) -> String {
    let mut s = String::from("Repeated corrections:\n");
    for (i, p) in reps.iter().take(MAX_REPS).enumerate() {
        let snippet: String = p.chars().take(300).collect();
        s.push_str(&format!("{}. {}\n", i + 1, snippet.replace('\n', " ")));
    }
    if memories.is_empty() {
        s.push_str("\nExisting memories: (none)\n");
    } else {
        s.push_str("\nExisting memories (id — title):\n");
        for (id, title) in memories {
            let t: String = title.chars().take(120).collect();
            s.push_str(&format!("- {} — {}\n", id, t.replace('\n', " ")));
        }
    }
    s
}

/// Parse the model's JSON object. `memory_ids` is the allowed shortlist — an id
/// outside it (or unparseable) becomes `None`. Returns `None` when there is no
/// usable `text` (caller falls back to the representative snippet). Tolerates
/// surrounding prose / code fences by extracting the first `{ … }`.
pub fn parse_response(content: &str, memory_ids: &[uuid::Uuid]) -> Option<ClusterSummary> {
    let start = content.find('{')?;
    let end = content.rfind('}')?;
    if end <= start {
        return None;
    }
    let v: serde_json::Value = serde_json::from_str(&content[start..=end]).ok()?;
    let text = v
        .get("text")
        .and_then(|t| t.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())?;
    let suggestion = v
        .get("suggestion")
        .and_then(|t| t.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);
    let memory_id = v
        .get("memory_id")
        .and_then(|m| m.as_str())
        .and_then(|s| uuid::Uuid::parse_str(s.trim()).ok())
        .filter(|id| memory_ids.contains(id));
    Some(ClusterSummary { text: text.to_string(), suggestion, memory_id })
}

/// Summarize one cluster via the gateway `reasoning` chain. `None` ⇒ caller falls
/// back. Graceful: never errors out.
pub async fn summarize_cluster(
    gateway: &Gateway,
    reps: &[&str],
    memories: &[(uuid::Uuid, String)],
) -> Option<ClusterSummary> {
    use gateway::types::capability::Capability;
    use gateway::types::request::*;
    let memory_ids: Vec<uuid::Uuid> = memories.iter().map(|(id, _)| *id).collect();
    let request = InferenceRequest {
        capability: Capability::TextChat,
        model: None,
        router: None,
        chain: Some("reasoning".into()),
        payload: Payload::Chat {
            messages: vec![Message::text(MessageRole::User, build_user_message(reps, memories))],
            system: Some(SYSTEM.to_string()),
            max_tokens: Some(MAX_TOKENS),
            temperature: None,
            tools: Vec::new(),
        },
        budget: None,
    };
    match gateway.execute(&request).await {
        Ok(resp) if resp.success => {
            let parsed = resp.content.as_deref().and_then(|c| parse_response(c, &memory_ids));
            if parsed.is_none() {
                tracing::warn!("corrections_llm: unparseable summary — cluster falls back to snippet");
            }
            parsed
        }
        Ok(_) => None,
        Err(e) => {
            tracing::debug!(error = %e, "corrections_llm: gateway unavailable — cluster falls back");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_response_full_object() {
        let mid = uuid::Uuid::new_v4();
        let c = format!(
            r#"{{"text":"Use $state for reactive locals","suggestion":"Reinforce the svelte5 memory","memory_id":"{mid}"}}"#
        );
        let got = parse_response(&c, &[mid]).unwrap();
        assert_eq!(got.text, "Use $state for reactive locals");
        assert_eq!(got.suggestion.as_deref(), Some("Reinforce the svelte5 memory"));
        assert_eq!(got.memory_id, Some(mid));
    }

    #[test]
    fn parse_response_tolerates_fences_and_null_memory() {
        let c = "```json\n{\"text\":\"Revert unwanted edits\",\"suggestion\":null,\"memory_id\":null}\n```";
        let got = parse_response(c, &[]).unwrap();
        assert_eq!(got.text, "Revert unwanted edits");
        assert_eq!(got.suggestion, None);
        assert_eq!(got.memory_id, None);
    }

    #[test]
    fn parse_response_drops_memory_id_not_in_shortlist() {
        let other = uuid::Uuid::new_v4();
        let c = format!(r#"{{"text":"x","memory_id":"{other}"}}"#);
        let got = parse_response(&c, &[uuid::Uuid::new_v4()]).unwrap();
        assert_eq!(got.memory_id, None, "id outside shortlist rejected");
    }

    #[test]
    fn parse_response_none_without_text() {
        assert_eq!(parse_response(r#"{"suggestion":"do x"}"#, &[]), None);
        assert_eq!(parse_response("not json", &[]), None);
        assert_eq!(parse_response("", &[]), None);
    }

    #[test]
    fn build_user_message_bounds_and_lists_memories() {
        let id = uuid::Uuid::new_v4();
        let msg = build_user_message(&["fix it", "revert that"], &[(id, "svelte5 state".into())]);
        assert!(msg.contains("1. fix it"));
        assert!(msg.contains("2. revert that"));
        assert!(msg.contains(&id.to_string()));
        let long = "x".repeat(500);
        let msg = build_user_message(&[&long], &[]);
        assert_eq!(msg.matches('x').count(), 300, "prompt bounded to 300 chars");
        assert!(msg.contains("(none)"));
    }
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p senseid --lib tasks::handlers::corrections_llm::tests`
Expected: PASS (5 tests).

- [ ] **Step 4: Commit**

```bash
git add crates/senseid/src/tasks/handlers/corrections_llm.rs crates/senseid/src/tasks/handlers/mod.rs
git commit -m "feat: corrections_llm — graceful per-cluster canonicalization (text/suggestion/memory)"
```

---

## Task 4: pg_store data access

**Files:**
- Modify: `crates/senseid/src/db/pg_store.rs`
- Test: inline `#[tokio::test]` (DB-gated, mirrors existing `pattern_upsert_*` tests)

- [ ] **Step 1: Write the failing integration test**

In `crates/senseid/src/db/pg_store.rs`, inside the existing `#[cfg(test)] mod tests`, after the `pattern_upsert_updates_existing` test (~line 4072), add:

```rust
    // ── Corrections aggregation tests ──────────────────────────────────

    #[tokio::test]
    async fn correction_upsert_is_idempotent_by_signature() {
        let s = pg_store().await;
        let p = uuid::Uuid::new_v4();
        let sig = format!("corr-test-{}", uuid::Uuid::new_v4());
        let row = crate::corrections::CorrectionRow {
            signature: sig.clone(),
            text: "Use $state for reactive locals".into(),
            suggestion: Some("Reinforce the svelte5 memory".into()),
            count: 3,
            project_ids: vec![p],
            last_seen: chrono::Utc::now(),
            memory_id: None,
            instances: serde_json::json!([{"session_id": "s1", "ts": 1, "prompt": "use $state"}]),
        };
        let id1 = s.upsert_correction(&row).await.unwrap();
        let mut row2 = row.clone();
        row2.count = 4;
        let id2 = s.upsert_correction(&row2).await.unwrap();
        assert_eq!(id1, id2, "same signature updates the same row");

        let global = s.list_corrections().await.unwrap();
        let found = global["corrections"].as_array().unwrap().iter()
            .find(|c| c["id"] == id1.to_string()).unwrap().clone();
        assert_eq!(found["count"], 4);
        assert_eq!(found["text"], "Use $state for reactive locals");
        assert!(found["projects"].as_array().unwrap().iter().any(|pr| pr["id"] == p.to_string()),
            "project resolved into projects[] (requires the project row to exist)");

        // prune everything except a non-existent signature → our row is deleted
        let pruned = s.delete_corrections_not_in(&["corr-nope".to_string()]).await.unwrap();
        assert!(pruned >= 1);
    }
```

Note: the `projects[]` assertion requires a real project row for `p`. If the `pg_store()` helper has a `create_test_project` helper, create `p` via it first; otherwise relax the `projects` assertion to `found["projects"].is_array()` (the `json_agg` returns `[]` when the id isn't a known project). Use whichever matches the existing test helpers — check `create_test_folder`/`create_test_project` in the test module.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p senseid --lib db::pg_store::tests::correction_upsert_is_idempotent_by_signature`
Expected: FAIL to compile — `no method named upsert_correction` / `list_corrections` / `delete_corrections_not_in`.

- [ ] **Step 3: Implement the data-access methods**

In `crates/senseid/src/db/pg_store.rs`, add these methods inside the `impl PgStore` block, near the detected-patterns methods (after `get_patterns_for_generation`, ~line 1491):

```rust
    // ── Corrections aggregation (#65 step 5) ─────────────────────────────────

    /// All captured user prompts across every project: (project_id, project_name,
    /// session_id, ts_ms, prompt). Ordered by ts so the handler's clustering seeds
    /// on the earliest member. The handler filters to corrections.
    pub async fn get_all_user_prompts(
        &self,
    ) -> Result<Vec<(uuid::Uuid, String, String, i64, String)>, String> {
        let rows: Vec<(uuid::Uuid, String, String, i64, String)> = sqlx_core::query_as::query_as(
            "SELECT s.project_id, COALESCE(p.name, ''), ae.session_id, ae.ts, ae.payload->>'prompt'
               FROM activity.assistant_events ae
               JOIN activity.sessions s ON s.client_session_id = ae.session_id
               JOIN sensei.projects p ON p.id = s.project_id
              WHERE ae.event_type = 'UserPromptSubmit'
                AND ae.payload->>'prompt' IS NOT NULL
                AND s.project_id IS NOT NULL
              ORDER BY ae.ts",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Active memories offered to the corrections summarizer for linking: (id,
    /// title). Bounded; most-recent first.
    pub async fn get_learned_memories_for_matching(
        &self,
    ) -> Result<Vec<(uuid::Uuid, String)>, String> {
        let rows: Vec<(uuid::Uuid, String)> = sqlx_core::query_as::query_as(
            "SELECT id, title FROM sensei.memories
              WHERE status = 'active'
              ORDER BY created_at DESC
              LIMIT 100",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Upsert one aggregated correction, keyed by its stable `signature` (so `id`
    /// stays constant across re-derivations).
    pub async fn upsert_correction(
        &self,
        row: &crate::corrections::CorrectionRow,
    ) -> Result<uuid::Uuid, String> {
        let r: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO inference.corrections
                (signature, text, suggestion, count, project_ids, last_seen, memory_id, instances, modified_at)
             VALUES($1, $2, $3, $4, $5, $6, $7, $8, now())
             ON CONFLICT(signature) DO UPDATE SET
               text = EXCLUDED.text,
               suggestion = EXCLUDED.suggestion,
               count = EXCLUDED.count,
               project_ids = EXCLUDED.project_ids,
               last_seen = EXCLUDED.last_seen,
               memory_id = EXCLUDED.memory_id,
               instances = EXCLUDED.instances,
               modified_at = now()
             RETURNING id",
        )
        .bind(&row.signature)
        .bind(&row.text)
        .bind(&row.suggestion)
        .bind(row.count)
        .bind(&row.project_ids)
        .bind(row.last_seen)
        .bind(&row.memory_id)
        .bind(&row.instances)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(r.0)
    }

    /// Delete corrections whose signature is not in `keep`. With an empty slice
    /// this clears the table (no corrections currently recur). Returns row count.
    pub async fn delete_corrections_not_in(&self, keep: &[String]) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "DELETE FROM inference.corrections WHERE signature <> ALL($1)",
        )
        .bind(keep)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    /// Global corrections list (camelCase, projects resolved to {id, name}).
    pub async fn list_corrections(&self) -> Result<serde_json::Value, String> {
        self.query_corrections(None).await
    }

    /// Corrections touching a specific project.
    pub async fn list_corrections_for_project(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<serde_json::Value, String> {
        self.query_corrections(Some(project_id)).await
    }

    /// Shared read: optionally filter to a project, resolve `project_ids` → a JSON
    /// array of {id, name}. The projects array is aggregated as text and parsed in
    /// Rust (the codebase avoids decoding json columns directly).
    async fn query_corrections(
        &self,
        project_filter: Option<&uuid::Uuid>,
    ) -> Result<serde_json::Value, String> {
        let rows: Vec<(
            uuid::Uuid,
            String,
            i32,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<uuid::Uuid>,
            Option<String>,
            String,
        )> = sqlx_core::query_as::query_as(
            "SELECT c.id, c.text, c.count, c.last_seen, c.memory_id, c.suggestion,
                    COALESCE((SELECT json_agg(json_build_object('id', p.id, 'name', p.name) ORDER BY p.name)
                              FROM sensei.projects p WHERE p.id = ANY(c.project_ids)), '[]'::json)::text
               FROM inference.corrections c
              WHERE ($1::uuid IS NULL OR $1 = ANY(c.project_ids))
              ORDER BY c.count DESC, c.last_seen DESC NULLS LAST",
        )
        .bind(project_filter)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let out: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|(id, text, count, last_seen, memory_id, suggestion, projects_json)| {
                let projects: serde_json::Value =
                    serde_json::from_str(&projects_json).unwrap_or_else(|_| serde_json::json!([]));
                serde_json::json!({
                    "id": id,
                    "text": text,
                    "count": count,
                    "lastSeen": last_seen.map(|t| t.to_rfc3339()),
                    "projects": projects,
                    "memoryId": memory_id,
                    "suggestion": suggestion,
                })
            })
            .collect();
        Ok(serde_json::json!({ "corrections": out }))
    }
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p senseid --lib db::pg_store::tests::correction_upsert_is_idempotent_by_signature`
Expected: PASS. (Requires the `sensei_test` DB with the table from Task 1.)

- [ ] **Step 5: Commit**

```bash
git add crates/senseid/src/db/pg_store.rs
git commit -m "feat(db): corrections aggregation data access (upsert/prune/list + prompts/memories)"
```

---

## Task 5: Orchestration handler + task wiring

**Files:**
- Create: `crates/senseid/src/tasks/handlers/corrections.rs`
- Modify: `crates/senseid/src/tasks/handlers/mod.rs` (declare + re-export)
- Modify: `crates/senseid/src/tasks/mod.rs` (TaskKind variant + Display + watchdog)
- Modify: `crates/senseid/src/tasks/executor.rs` (route)
- Test: inline `#[cfg(test)] mod tests` for the pure `build_rows`

- [ ] **Step 1: Add the TaskKind variant**

In `crates/senseid/src/tasks/mod.rs`:

(a) In `enum TaskKind` (after `BackfillTranscriptFile`, ~line 54):

```rust
    /// Global: cluster recurring corrective prompts across all projects into
    /// inference.corrections (analyzer #65 step 5). Enqueued once per scheduler tick.
    AggregateCorrections,
```

(b) In `impl Display for TaskKind` (after the `BackfillTranscriptFile` arm, ~line 81):

```rust
            Self::AggregateCorrections => write!(f, "aggregate_corrections"),
```

(c) In `watchdog_timeout`, add `AggregateCorrections` to the 600s arm alongside `AnalyzeProject` (~line 122):

```rust
            | TaskKind::DetectCommunities
            | TaskKind::AnalyzeProject
            | TaskKind::AggregateCorrections => Duration::from_secs(600),
```

- [ ] **Step 2: Declare + route the handler**

(a) In `crates/senseid/src/tasks/handlers/mod.rs`, add near `mod analyze;`:

```rust
mod corrections;
```

and near `pub use analyze::analyze_project;`:

```rust
pub use corrections::aggregate_corrections;
```

(b) In `crates/senseid/src/tasks/executor.rs`, add to the `match task.kind` block (after the `AnalyzeProject` arm, ~line 105):

```rust
            TaskKind::AggregateCorrections => handlers::aggregate_corrections(ctx, task).await,
```

- [ ] **Step 3: Write the handler with a failing `build_rows` test**

Create `crates/senseid/src/tasks/handlers/corrections.rs`:

```rust
//! Corrections aggregation handler (#65 step 5). Global: pull every user prompt,
//! keep corrections (regex recall → LLM precision), embed them (lexical fallback),
//! deterministically cluster, summarize each surviving cluster (graceful), and
//! upsert idempotently by signature (then prune stale signatures).

use super::super::executor::TaskContext;
use super::super::Task;
use super::analyze::correction_signal;
use super::corrections_llm::{summarize_cluster, ClusterSummary};
use super::prompt_classify::{classify_batch, PromptClass};
use crate::corrections::{self, Cluster, CorrItem, CorrectionRow, CORRECTION_CLUSTER_MIN, SIMILARITY_THRESHOLD};

/// Prompts embedded per gateway call.
const EMBED_BATCH: usize = 64;
/// Per-batch embedding wall-clock cap — a stalled backend must not wedge the task.
const EMBED_TIMEOUT_SECS: u64 = 30;

/// Pure: assemble upsert rows from clusters + their (aligned) summaries. Drops
/// clusters below `CORRECTION_CLUSTER_MIN`. `summaries[i]` corresponds to
/// `clusters[i]`; `None` ⇒ snippet fallback.
pub fn build_rows(
    clusters: &[Cluster],
    summaries: &[Option<ClusterSummary>],
    items: &[CorrItem],
) -> Vec<CorrectionRow> {
    let mut rows = Vec::new();
    for (c, summary) in clusters.iter().zip(summaries.iter()) {
        if c.count < CORRECTION_CLUSTER_MIN {
            continue;
        }
        let (text, suggestion, memory_id) = match summary {
            Some(s) => (s.text.clone(), s.suggestion.clone(), s.memory_id),
            None => (c.representative_text.clone(), None, None),
        };
        let instances = serde_json::Value::Array(
            c.member_idxs
                .iter()
                .filter_map(|&i| items.get(i))
                .map(|it| {
                    serde_json::json!({
                        "project_id": it.project_id,
                        "session_id": it.session_id,
                        "ts": it.ts,
                        "prompt": corrections::normalize(&it.prompt),
                    })
                })
                .collect(),
        );
        rows.push(CorrectionRow {
            signature: c.signature.clone(),
            text,
            suggestion,
            count: c.count as i32,
            project_ids: c.project_ids.clone(),
            last_seen: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(c.last_seen_ms)
                .unwrap_or_default(),
            memory_id,
            instances,
        });
    }
    rows
}

/// Embed each item's normalized prompt via the 384-dim `embed` chain. `None` ⇒ no
/// embed model / failure / count mismatch (caller uses the lexical fallback).
async fn embed_items(ctx: &TaskContext, items: &[CorrItem]) -> Option<Vec<Vec<f32>>> {
    use gateway::types::capability::Capability;
    use gateway::types::request::{InferenceRequest, Payload};
    let mut out: Vec<Vec<f32>> = Vec::with_capacity(items.len());
    for chunk in items.chunks(EMBED_BATCH) {
        let texts: Vec<String> = chunk.iter().map(|it| corrections::normalize(&it.prompt)).collect();
        let request = InferenceRequest {
            capability: Capability::TextEmbed,
            model: None,
            router: None,
            chain: Some("embed".to_string()),
            payload: Payload::Embed { texts },
            budget: None,
        };
        match tokio::time::timeout(
            std::time::Duration::from_secs(EMBED_TIMEOUT_SECS),
            ctx.app_state.gateway.execute(&request),
        )
        .await
        {
            Ok(Ok(resp)) => {
                let embs = resp.embeddings.unwrap_or_default();
                if embs.len() != chunk.len() {
                    return None;
                }
                out.extend(embs);
            }
            _ => return None,
        }
    }
    Some(out)
}

/// Global corrections aggregation. Idempotent; degrades gracefully without models.
pub async fn aggregate_corrections(ctx: &TaskContext, _task: &Task) -> Result<u32, String> {
    // 1. Pull all user prompts, keep regex-recall corrections.
    let all = ctx.pg().get_all_user_prompts().await?;
    let mut candidates: Vec<CorrItem> = all
        .into_iter()
        .filter(|(_, _, _, _, prompt)| correction_signal(prompt).is_some())
        .map(|(project_id, _pname, session_id, ts, prompt)| CorrItem { project_id, session_id, ts, prompt })
        .collect();

    if candidates.is_empty() {
        let pruned = ctx.pg().delete_corrections_not_in(&[]).await.unwrap_or(0);
        tracing::info!("aggregate_corrections: no corrective prompts; {pruned} pruned");
        return Ok(0);
    }

    // 2. LLM precision pass (graceful): drop regex false positives + principles.
    let texts: Vec<&str> = candidates.iter().map(|c| c.prompt.as_str()).collect();
    let refined = classify_batch(&ctx.app_state.gateway, &texts).await;
    let mut items: Vec<CorrItem> = candidates
        .drain(..)
        .enumerate()
        .filter(|(i, _)| {
            !matches!(
                refined.get(*i).copied().flatten(),
                Some(PromptClass::Principle) | Some(PromptClass::Neither)
            )
        })
        .map(|(_, c)| c)
        .collect();

    // Deterministic order: earliest ts seeds each cluster (session tiebreak).
    items.sort_by(|a, b| a.ts.cmp(&b.ts).then_with(|| a.session_id.cmp(&b.session_id)));

    // 3. Cluster (embed → lexical fallback).
    let clusters = match embed_items(ctx, &items).await {
        Some(embeddings) => corrections::cluster(&items, &embeddings, SIMILARITY_THRESHOLD),
        None => {
            tracing::warn!("aggregate_corrections: no embeddings — lexical fallback");
            corrections::lexical_cluster(&items)
        }
    };

    // 4. Per-cluster summary (graceful), only for clusters that will surface.
    let memories = ctx.pg().get_learned_memories_for_matching().await.unwrap_or_default();
    let mut summaries: Vec<Option<ClusterSummary>> = Vec::with_capacity(clusters.len());
    for c in &clusters {
        if c.count < CORRECTION_CLUSTER_MIN {
            summaries.push(None);
            continue;
        }
        let reps: Vec<&str> = c
            .member_idxs
            .iter()
            .take(5)
            .filter_map(|&i| items.get(i))
            .map(|it| it.prompt.as_str())
            .collect();
        summaries.push(summarize_cluster(&ctx.app_state.gateway, &reps, &memories).await);
    }

    // 5. Upsert + prune.
    let rows = build_rows(&clusters, &summaries, &items);
    let mut keep: Vec<String> = Vec::with_capacity(rows.len());
    for row in &rows {
        keep.push(row.signature.clone());
        if let Err(e) = ctx.pg().upsert_correction(row).await {
            tracing::warn!(error = %e, signature = %row.signature, "aggregate_corrections: upsert failed");
        }
    }
    let pruned = ctx.pg().delete_corrections_not_in(&keep).await.unwrap_or(0);
    tracing::info!("aggregate_corrections: {} upserted, {} pruned", rows.len(), pruned);
    Ok(rows.len() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(pid: uuid::Uuid, ts: i64, prompt: &str) -> CorrItem {
        CorrItem { project_id: pid, session_id: "s".into(), ts, prompt: prompt.into() }
    }

    #[test]
    fn build_rows_drops_singletons_and_uses_summary_then_snippet() {
        let p = uuid::Uuid::new_v4();
        let items = vec![item(p, 100, "use $state"), item(p, 200, "use $state please"), item(p, 300, "lone")];
        let clusters = vec![
            Cluster {
                signature: "corr-a".into(),
                representative_text: "use state".into(),
                count: 2,
                last_seen_ms: 200,
                project_ids: vec![p],
                member_idxs: vec![0, 1],
            },
            Cluster {
                signature: "corr-b".into(),
                representative_text: "lone".into(),
                count: 1,
                last_seen_ms: 300,
                project_ids: vec![p],
                member_idxs: vec![2],
            },
        ];
        let summaries = vec![
            Some(ClusterSummary { text: "Use $state".into(), suggestion: Some("reinforce".into()), memory_id: None }),
            None,
        ];
        let rows = build_rows(&clusters, &summaries, &items);
        assert_eq!(rows.len(), 1, "singleton dropped");
        assert_eq!(rows[0].signature, "corr-a");
        assert_eq!(rows[0].text, "Use $state", "summary text used");
        assert_eq!(rows[0].count, 2);
        assert_eq!(rows[0].instances.as_array().unwrap().len(), 2);
    }

    #[test]
    fn build_rows_falls_back_to_snippet_without_summary() {
        let p = uuid::Uuid::new_v4();
        let items = vec![item(p, 100, "revert that"), item(p, 200, "revert that please")];
        let clusters = vec![Cluster {
            signature: "corr-c".into(),
            representative_text: "revert that".into(),
            count: 2,
            last_seen_ms: 200,
            project_ids: vec![p],
            member_idxs: vec![0, 1],
        }];
        let rows = build_rows(&clusters, &[None], &items);
        assert_eq!(rows[0].text, "revert that", "snippet fallback");
        assert_eq!(rows[0].suggestion, None);
        assert_eq!(rows[0].memory_id, None);
    }
}
```

- [ ] **Step 4: Run the pure tests to verify they pass**

Run: `cargo test -p senseid --lib tasks::handlers::corrections::tests`
Expected: PASS (2 tests).

- [ ] **Step 5: Build the crate to confirm wiring compiles**

Run: `cargo build -p senseid`
Expected: builds cleanly (TaskKind variant, executor route, handler re-export all resolve).

- [ ] **Step 6: Commit**

```bash
git add crates/senseid/src/tasks/handlers/corrections.rs crates/senseid/src/tasks/handlers/mod.rs crates/senseid/src/tasks/mod.rs crates/senseid/src/tasks/executor.rs
git commit -m "feat: aggregate_corrections handler + AggregateCorrections task wiring"
```

---

## Task 6: Scheduler — enqueue the global task once per tick

**Files:**
- Modify: `crates/senseid/src/tasks/analyzer_scheduler.rs`

- [ ] **Step 1: Enqueue when any project was due**

In `crates/senseid/src/tasks/analyzer_scheduler.rs`, change the body of the `run` loop's match arm (~lines 68–77) from:

```rust
            Ok(activity) => {
                for pid in projects_due(&activity, &mut watermark) {
                    queue
                        .enqueue(Task::new(TaskKind::AnalyzeProject, "", &pid.to_string()))
                        .await;
                }
            }
```

to:

```rust
            Ok(activity) => {
                let due = projects_due(&activity, &mut watermark);
                let any_due = !due.is_empty();
                for pid in due {
                    queue
                        .enqueue(Task::new(TaskKind::AnalyzeProject, "", &pid.to_string()))
                        .await;
                }
                // Corrections cluster globally across projects, so derive once per
                // tick after the per-project analyses — only when something changed.
                if any_due {
                    queue
                        .enqueue(Task::new(TaskKind::AggregateCorrections, "", ""))
                        .await;
                }
            }
```

Note: no new unit test here. The selection logic (`projects_due`) is already covered by `projects_due_picks_new_then_only_advanced`; the added branch is a one-line `if any_due` glue over a real `TaskQueue` (verified in Task 8's live check). This is a deliberate choice, not an omission.

- [ ] **Step 2: Build to confirm it compiles**

Run: `cargo build -p senseid`
Expected: builds cleanly.

- [ ] **Step 3: Commit**

```bash
git add crates/senseid/src/tasks/analyzer_scheduler.rs
git commit -m "feat: enqueue AggregateCorrections once per analyzer tick when projects are due"
```

---

## Task 7: API endpoints

**Files:**
- Create: `crates/senseid/src/api/handlers/corrections.rs`
- Modify: `crates/senseid/src/api/handlers/mod.rs` (declare module)
- Modify: `crates/senseid/src/api/routes.rs` (import + two routes)

- [ ] **Step 1: Write the read handlers**

Create `crates/senseid/src/api/handlers/corrections.rs`:

```rust
//! Corrections aggregation read API (#65 step 5).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::api::state::AppState;

/// GET /api/corrections — global recurring-corrections list.
pub(crate) async fn list_corrections(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let data = state.pg.list_corrections().await.map_err(|e| {
        tracing::error!("list_corrections: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(data))
}

/// GET /api/projects/{id}/corrections — corrections touching a project.
pub(crate) async fn project_corrections(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let project_id = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let data = state
        .pg
        .list_corrections_for_project(&project_id)
        .await
        .map_err(|e| {
            tracing::error!("project_corrections: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(data))
}
```

- [ ] **Step 2: Declare the module**

In `crates/senseid/src/api/handlers/mod.rs`, add (after `pub(crate) mod knowledge;`):

```rust
pub(crate) mod corrections;
```

- [ ] **Step 3: Register the routes**

In `crates/senseid/src/api/routes.rs`:

(a) Add the import next to the other handler imports (~line 17):

```rust
use crate::api::handlers::corrections;
```

(b) Add the two routes near the other observatory/project routes (after the `/api/projects/{id}/maturity` route, ~line 80):

```rust
        .route("/api/corrections", get(corrections::list_corrections))
        .route("/api/projects/{id}/corrections", get(corrections::project_corrections))
```

- [ ] **Step 4: Build to confirm it compiles**

Run: `cargo build -p senseid`
Expected: builds cleanly.

- [ ] **Step 5: Commit**

```bash
git add crates/senseid/src/api/handlers/corrections.rs crates/senseid/src/api/handlers/mod.rs crates/senseid/src/api/routes.rs
git commit -m "feat(api): GET /api/corrections + /api/projects/{id}/corrections"
```

---

## Task 8: Zero-errors gate + live verification

**Files:** none (verification + final commit)

- [ ] **Step 1: Run the zero-errors policy (lint + full tests)**

Run: `make test`
Expected: all Rust + app tests pass, 0 failures (includes the new `corrections`, `corrections_llm`, handler, and DB tests).

If lint is separate in this repo, also run the project's clippy/lint target and confirm 0 warnings on the new files.

- [ ] **Step 2: Build + install the daemon against local DDL**

Run: `make crates-debug && make install-debug`
Expected: builds and overlays the debug daemon.

Ensure the runtime DB has the table (Task 1, Step 2). If iterating on DDL locally, launch with `SENSEI_DDL_DIR=$(pwd)/database` so the daemon resolves the local schema.

- [ ] **Step 3: Trigger the task and verify rows**

Enqueue a run (either wait for a scheduler tick after session activity, or enqueue directly if an admin endpoint exists). Then:

Run: `psql -p 7744 -d sensei -c "SELECT signature, count, array_length(project_ids,1) AS projects, left(text,60) AS text, (suggestion IS NOT NULL) AS has_suggestion, (memory_id IS NOT NULL) AS has_memory FROM inference.corrections ORDER BY count DESC LIMIT 20;"`
Expected: rows with `count >= 2`, a `text`, and (when a chat model is configured) `has_suggestion = t`.

- [ ] **Step 4: Verify the endpoints**

Run: `curl -s http://127.0.0.1:7744/api/corrections | jq '.corrections[0]'`
Expected: an object shaped `{ id, text, count, lastSeen, projects: [{id,name}], memoryId, suggestion }`.

Run: `curl -s http://127.0.0.1:7744/api/projects/<a-project-id>/corrections | jq '.corrections | length'`
Expected: a number ≥ 0; spot-check that returned corrections include that project in `projects[]`.

- [ ] **Step 5: Verify idempotency**

Trigger the task a second time (no new sessions), then:

Run: `psql -p 7744 -d sensei -c "SELECT count(*) FROM inference.corrections;"`
Expected: the row count is stable across the two runs (no duplicates) — confirms signature-based upsert + prune.

- [ ] **Step 6: Update the backlog**

In `docs/backlog.md`, add a line under section 5 noting corrections aggregation shipped (commit hash), mirroring how the other analyzer sub-items are recorded under #65.

- [ ] **Step 7: Final commit**

```bash
git add docs/backlog.md
git commit -m "docs: corrections aggregation shipped (analyzer #65 step 5)"
```

---

## Self-Review

**Spec coverage:**
- New `inference.corrections` table (full shape incl. suggestion + memoryId) → Task 1. ✓
- Deterministic seed-derived signature + idempotent upsert + prune → Tasks 2, 4 (`signature`, `upsert_correction` ON CONFLICT, `delete_corrections_not_in`). ✓
- Global clustering, project-tagged (`project_ids[]`) → Tasks 2 (`cluster`/`Cluster.project_ids`) + 4 (`get_all_user_prompts` global). ✓
- Embedding-cluster → per-cluster LLM, with lexical + snippet graceful fallback → Tasks 2 (`lexical_cluster`), 3 (`summarize_cluster` graceful), 5 (`embed_items` None → lexical; build_rows snippet fallback). ✓
- Reuse L1 classifiers (regex recall → `classify_batch`) → Task 5. ✓
- Recurrence threshold `count >= CORRECTION_CLUSTER_MIN` (2) → Task 2 const + Task 5 `build_rows` + summary gating. ✓
- Dedicated global task once per scheduler tick → Tasks 5 (TaskKind/executor) + 6 (scheduler). ✓
- `GET /api/corrections` + `/api/projects/{id}/corrections`, camelCase, projects resolved to {id,name} → Tasks 4 (`query_corrections`) + 7. ✓
- Graceful degradation / no silent errors (logged warnings) → Tasks 3 + 5. ✓
- Out of scope (based_on wiring, dismiss lifecycle, UI) → not implemented, as specified. ✓

**Placeholder scan:** No TBD/TODO; every code step shows complete code; every command shows expected output. The one "no test here" note (Task 6) is a justified decision, not a placeholder.

**Type consistency:** `CorrItem` and `Cluster` (Task 2) are used identically in Tasks 5; `CorrectionRow` (Task 2) is produced by `build_rows` (Task 5) and consumed by `upsert_correction` (Task 4); `ClusterSummary` (Task 3) is produced by `summarize_cluster` and consumed by `build_rows`. Method names (`get_all_user_prompts`, `get_learned_memories_for_matching`, `upsert_correction`, `delete_corrections_not_in`, `list_corrections`, `list_corrections_for_project`) match between Task 4 (definitions) and Tasks 5/7 (call sites). `aggregate_corrections` matches across handler (Task 5), `mod.rs` re-export, and executor route. `TaskKind::AggregateCorrections` matches across enum/Display/watchdog/executor/scheduler.

**Known build-time risks to watch (verify during execution, not assumed):**
- Binding `Vec<uuid::Uuid>` → `uuid[]` and `&[String]` → `text[]` via sqlx: standard, but confirm at Task 4 compile. If array binding needs a different form in this sqlx setup, adapt (e.g. `&row.project_ids[..]`).
- Column names `text` and `count` are non-reserved in PostgreSQL and valid unquoted; queries qualify them (`c.text`, `c.count`). If any tooling objects, the fix is local to Task 1 + Task 4 SQL.
- The DB test's `projects[]` assertion needs a real project row — relax per the note in Task 4, Step 1 if no `create_test_project` helper exists.
