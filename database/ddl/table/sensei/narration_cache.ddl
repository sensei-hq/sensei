set search_path to sensei, extensions;

-- Persisted mentor-voice copy for insight cards. The narration-cache pipeline
-- (crates/senseid/src/analysis/narration_cache.rs) routes structured facts
-- through a small local model (embedded gemma4 via the sensei gateway) and
-- caches the result here keyed on (kind, facts_hash). Same facts → same copy
-- indefinitely, so the wire path never blocks on inference in steady state.
--
-- `kind` is the stable snake_case insight key (e.g. tool_dormant). `facts_hash`
-- is sha256(kind + canonical_json(facts)); any facts change makes a new key.
-- `title` + `detail` are the model's validated output (or a regenerated one).
-- `model_provider` / `model_id` record which leg produced the copy (NULL when
-- unknown). Rows are never deleted synchronously — a daily maintenance task
-- evicts rows with last_used_at older than 30 days. On read, last_used_at is
-- bumped so hot copy stays warm.
create table if not exists narration_cache (
  kind           text        not null
, facts_hash     text        not null
, title          text        not null
, detail         text        not null
, model_provider text
, model_id       text
, generated_at   timestamptz not null default now()
, last_used_at   timestamptz not null default now()
, primary key (kind, facts_hash)
);

-- Eviction sweep: the daily maintenance task deletes cold rows ordered by
-- last_used_at, so index it for cheap range scans.
create index if not exists narration_cache_last_used_idx
    on narration_cache(last_used_at);

comment on table narration_cache is
'Persisted mentor-voice insight copy produced by the narration-cache pipeline.
Keyed on (kind, facts_hash) so identical facts reuse the same generated copy;
the static per-call-site fallback template is used only on cold start,
timeout, or model failure. Rows evict after 30 days of no use.';

comment on column narration_cache.kind
     is 'Stable snake_case insight key (e.g. tool_dormant) — InsightKind::as_str.';
comment on column narration_cache.facts_hash
     is 'sha256(kind + canonical_json(facts)); any facts change makes a new key.';
comment on column narration_cache.title
     is 'Model-generated card title (validated against the char + voice limits).';
comment on column narration_cache.detail
     is 'Model-generated card detail (validated against the char + voice limits).';
comment on column narration_cache.model_provider
     is 'Provider/router that produced the copy (e.g. ollama), NULL when unknown.';
comment on column narration_cache.model_id
     is 'Model id that produced the copy (e.g. gemma4), NULL when unknown.';
comment on column narration_cache.generated_at
     is 'When this copy was generated (reset on regeneration).';
comment on column narration_cache.last_used_at
     is 'Bumped to now() on every cache read; drives the 30-day eviction sweep.';
