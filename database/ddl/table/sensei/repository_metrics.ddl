set search_path to sensei, extensions;

-- The value store — the single source of truth for every computed metric value.
--
-- RENAMED from `project_metrics`, which was a misnomer rather than a mismodel:
-- the identity index has keyed on repository_id (NOT project_id) since D1/D2, so
-- the table was already repository-grained and only its name disagreed.
-- Measured before the rename, across 15,389 rows: repository_id NULL = 0,
-- folder_id set = 0, session_id set = 0, and 0 rows whose project_id disagreed
-- with the folders mapping. So the rename dropped three columns that held
-- nothing and derived the fourth.
--
-- `sensei.project_metrics` survives as a VIEW over this table, restoring
-- project_id by lookup, so every dependent view and read path is unchanged.
create table if not exists repository_metrics (
  id            uuid          primary key default gen_random_uuid()
, metric_id     uuid          not null references sensei.metrics(id) on delete cascade
  -- NOT NULL now. It was nullable for "legacy rows pending migration"; that
  -- migration is done (0 nulls), so the column states the invariant instead of
  -- documenting an exception that no longer exists.
, repository_id uuid          not null references sensei.repositories(id) on delete cascade
, scope         metric_scope  not null default 'user'
, identity      text
  -- The persona `identity` resolves to. NULLABLE and DERIVED — never a
  -- replacement for the raw email above. Resolving at write time would be a
  -- destructive merge: each row came from `git log --author=<email>`, so
  -- combining two aliases must SUM, and a later persona reassignment would have
  -- no source left to recompute from. Keeping both makes re-attribution a
  -- re-derivation over immutable raw attribution.
, persona_id    uuid          references sensei.personas(id) on delete set null
, commit_sha    text
, computed_on   date          not null
, grain         metric_grain  not null
, value         numeric       not null
, props         jsonb         not null default '{}'
, source        metric_source not null default 'measured'
, modified_at   timestamptz   not null default now()
);

-- Identity of a stored value: one row per metric x repository x scope x identity
-- x commit_sha x date x grain. NULLS NOT DISTINCT so the null identity/commit_sha
-- rows (day-grain, scope=repo) collide rather than duplicate — this is the
-- upsert target the compute tasks conflict on, which is what makes a re-run
-- idempotent instead of doubling history.
create unique index if not exists repository_metrics_identity
    on repository_metrics (metric_id, repository_id, scope, identity, commit_sha, computed_on, grain)
    nulls not distinct;

-- Covering index for the repository_id FK: a repository delete cascades here,
-- and every read filters by repository.
create index if not exists repository_metrics_repository_idx
    on repository_metrics (repository_id, metric_id, computed_on);

-- Roll-ups group by persona; a partial index because only scope='user' rows
-- carry one (a repo-scope value has no author dimension).
create index if not exists repository_metrics_persona_idx
    on repository_metrics (persona_id, metric_id, computed_on) where persona_id is not null;

comment on table repository_metrics is
'The value store — every computed metric value, at repository + date grain. All
aggregation (week/month/quarter/trend) and the derived health score are views
over this one table.

Never-fabricate invariants:
  - No data => no row. Absence reads as "not yet measured," never a defaulted 0.
  - Ratios carry their parts. A ratio/pct row stores props.numerator +
    props.denominator so roll-ups re-derive (never average-of-averages).
  - Estimates tagged. source=''estimated'' is never rendered as truth; money-facing
    metrics write no row on a price miss (fail closed).';

comment on column repository_metrics.metric_id
     is 'FK to sensei.metrics — which registered metric this value is for.';
comment on column repository_metrics.repository_id
     is 'The grain. A value is attributed to the repository, regardless of which project(s) include it — which is why the same number is shared rather than recomputed per project.';
comment on column repository_metrics.scope
     is 'repo (whole repository, all authors) vs user (this identity''s own commits ∩ touched files).';
comment on column repository_metrics.identity
     is 'For scope=user, the git author email the value is attributed to. NULL for scope=repo. Kept as the RAW git assertion; a resolved persona FK is layered on top rather than replacing it, so re-attribution stays a re-derivation instead of a destructive edit.';
comment on column repository_metrics.commit_sha
     is 'The sampled commit a commit-cadence value was computed at (quality/churn/coverage). NULL for day-cadence values.';
comment on column repository_metrics.computed_on
     is 'The date the value is FOR (not when it was computed).';
comment on column repository_metrics.grain
     is 'Grain of the row. Only ''daily'' is written today.';
comment on column repository_metrics.value
     is 'The computed value. Its meaning is defined by metrics.type/unit/direction.';
comment on column repository_metrics.props
     is 'Extensible parts: numerator/denominator, session_count, correction_count, low_n, evidence ids.';
comment on column repository_metrics.source
     is 'Provenance: measured | estimated. estimated is never rendered as truth.';
