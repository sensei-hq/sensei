set search_path to sensei, extensions;

create table if not exists consolidated_rulesets (
  id           uuid        primary key default gen_random_uuid()
, scope        text        not null
, version      integer     not null
, content      text        not null
, conflicts    jsonb       not null default '[]'
, model        text
, source_hash  text        not null
, status       text        not null default 'proposed'
, created_at   timestamptz not null default now()
, constraint consolidated_rulesets_scope_version unique (scope, version)
  -- Only `origin` from the shared vocabulary (sensei.entity_origin).
  --
  -- NOT `scope`: this table already has one, and it means something else — the
  -- GOVERNANCE scope the ruleset belongs to (it keys the unique constraint with
  -- `version`, and matches sensei.scopes). That is applicability, not visibility.
  -- Adding a second column of the same name would have been the exact confusion
  -- this vocabulary exists to end.
, origin       entity_origin not null default 'learned'
);

create index if not exists consolidated_rulesets_scope_status_idx
    on consolidated_rulesets(scope, status);

comment on table consolidated_rulesets is
'Governance Tier-2 output: an LLM-merged, human-approved ruleset for a scope.
Tier-1 resolution (resolve_rules_raw / resolve_global_rules) gathers + orders the
raw rules; Tier-2 asks a model to merge them into one coherent, deduped,
conflict-flagged markdown ruleset. Each consolidation is a kept version; the row
with status=approved for a scope is the active merged ruleset (it feeds
~/.sensei/rules.md). Versioning IS the history — no separate history table.';

comment on column consolidated_rulesets.scope
     is 'What was merged: ''global'' (always-on user/general rules) in v1; later a repo abs_path or namespace key.';
comment on column consolidated_rulesets.version
     is 'Monotonic per scope. New consolidation = max(version)+1.';
comment on column consolidated_rulesets.content
     is 'The merged markdown ruleset produced by the model.';
comment on column consolidated_rulesets.conflicts
     is 'Conflicts the model surfaced during the merge (JSON array). Empty when none/unextracted.';
comment on column consolidated_rulesets.model
     is 'Model that produced this merge (e.g. gemma4).';
comment on column consolidated_rulesets.source_hash
     is 'Hash of the Tier-1 input rules. A consolidate is skipped when the latest version already has this hash (no change to merge).';
comment on column consolidated_rulesets.status
     is 'proposed (awaiting approval) | approved (active, feeds rules.md) | superseded (an older approved version).';
comment on column consolidated_rulesets.created_at
     is 'When this version was produced.';
