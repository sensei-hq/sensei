set search_path to sensei, extensions;
create table if not exists project_metrics (
  id           uuid          primary key default gen_random_uuid()
, metric_id     uuid          not null references sensei.metrics(id) on delete cascade
, project_id    uuid          not null references sensei.projects(id) on delete cascade
, repository_id uuid          references sensei.repositories(id) on delete cascade
, scope         metric_scope  not null default 'user'
, identity      text
, commit_sha    text
, folder_id     uuid          references sensei.folders(id) on delete cascade
, session_id    uuid
, computed_on   date          not null
, grain         metric_grain  not null
, value         numeric       not null
, props         jsonb         not null default '{}'
, source        metric_source not null default 'measured'
, modified_at   timestamptz   not null default now()
);

-- Identity of a stored value: one row per metric x REPOSITORY x scope x identity x
-- commit_sha x date x grain (D1/D2 — repository is the grain; project_id is OUT of
-- the identity, kept only as a lookup column). NULLS NOT DISTINCT so the null
-- identity/commit_sha (day-grain, scope=repo) rows collide rather than duplicate —
-- the upsert target the compute tasks conflict on (idempotent re-run backfills).
-- Named _v2 deliberately: a `create unique index if not exists` with the OLD name
-- would SILENTLY no-op against the existing differently-columned index (G4), so the
-- old `project_metrics_identity` is dropped MANUALLY (dbd can't drop indexes) after
-- the old-grain rows are deleted, then this one is created.
create unique index if not exists project_metrics_identity_v2
    on project_metrics (metric_id, repository_id, scope, identity, commit_sha, computed_on, grain)
    nulls not distinct;

create index if not exists project_metrics_lookup
    on project_metrics (project_id, metric_id, computed_on);

-- Covering index for the folder_id FK (nullable) — a folder delete cascades here;
-- folder_id is only the 3rd column of project_metrics_identity, so it needs its own.
create index if not exists project_metrics_folder_idx
    on project_metrics (folder_id) where folder_id is not null;

-- Covering index for the repository_id FK (nullable): a repository delete cascades
-- here, and repo-grain reads filter by repository. (The identity-index swap that
-- adds repository_id/scope/identity/commit_sha to the uniqueness key — replacing
-- project_id — is applied in the metrics-engine step, in lockstep with the Rust
-- upsert's ON CONFLICT and after the old-grain rows are deleted.)
create index if not exists project_metrics_repository_idx
    on project_metrics (repository_id) where repository_id is not null;

comment on table project_metrics is
'The value store — the single source of truth for every computed metric value, at
project + (module) + (session) + date grain. All aggregation (week/month/quarter/
trend) and the derived health score are views over this one store.

Never-fabricate invariants:
  - No data => no row. Absence reads as "not yet measured," never a defaulted 0.
  - Grain explicit. grain=''session'' => session_id set; grain=''daily'' => session_id
    null. folder_id set only for module-scoped metrics; null = whole project.
  - Ratios carry their parts. A ratio/pct row stores props.numerator + props.denominator
    so roll-ups re-derive (never average-of-averages).
  - Estimates tagged. source=''estimated'' is never rendered as truth; money-facing
    metrics write no row on a price miss (fail closed).';

comment on column project_metrics.metric_id
     is 'FK to sensei.metrics — which registered metric this value is for.';
comment on column project_metrics.project_id
     is 'FK to sensei.projects — kept for lookup convenience (ON DELETE CASCADE), but NO LONGER part of the value identity: the same repository''s value is shared across every project that includes it. The identity keys on repository_id + scope + identity + commit_sha.';
comment on column project_metrics.repository_id
     is 'FK to sensei.repositories — the metric grain (D1/D2). The value is attributed to this repository regardless of which project(s) include it. NULL only on legacy rows pending migration.';
comment on column project_metrics.scope
     is 'repo (whole-repository, all authors) vs user (the local user''s own commits ∩ touched files). Replaces the folder_id-IS-NULL project/module overload.';
comment on column project_metrics.identity
     is 'The user identity a scope=user value is attributed to (the local git author/email). NULL for scope=repo.';
comment on column project_metrics.commit_sha
     is 'The sampled commit a commit-cadence value was computed at (quality/churn). NULL for day-cadence values.';
comment on column project_metrics.folder_id
     is 'FK to sensei.folders for a module-scoped (per-module) value. Null = whole project.';
comment on column project_metrics.session_id
     is 'Session this value is for, when grain=''session''. Null for aggregate (daily) rows.';
comment on column project_metrics.computed_on
     is 'The date the value is FOR (not when it was computed).';
comment on column project_metrics.grain
     is 'Grain of the row: session | daily.';
comment on column project_metrics.value
     is 'The computed value. Its meaning is defined by metrics.type/unit/direction.';
comment on column project_metrics.props
     is 'Extensible parts: numerator/denominator, session_count, correction_count, low_n, evidence ids.';
comment on column project_metrics.source
     is 'Provenance: measured | estimated. estimated is never rendered as truth.';
comment on column project_metrics.modified_at
     is 'Timestamp of the last modification to this row (set to now() on each upsert).';
