set search_path to sensei, extensions;

create type if not exists pattern_lifecycle
    as enum ('suggested', 'gap', 'rule');

create type if not exists pattern_severity
    as enum ('low', 'medium', 'high');

create table if not exists detected_patterns (
  id                       uuid              primary key default gen_random_uuid()
, folder_id                uuid              not null references sensei.folders(id) on delete cascade
, name                     text              not null
, family                   text
, lifecycle                pattern_lifecycle  not null default 'suggested'
, is_anti_pattern          boolean           not null default false
, severity                 pattern_severity
, confidence               numeric(3,2)
, instance_count           integer           not null default 0
, instances                jsonb             not null default '[]'
, evidence                 jsonb             not null default '[]'
, fix_pattern_id           uuid              references sensei.detected_patterns(id) on delete set null
, description              text
, example                  text
, enforcement              text
, tags                     text[]            not null default '{}'
, detected_at              timestamptz       not null default now()
, modified_at              timestamptz       not null default now()
, unique(folder_id, name, is_anti_pattern)
);

create index if not exists detected_patterns_folder_id_idx
    on detected_patterns(folder_id);

create index if not exists detected_patterns_lifecycle_idx
    on detected_patterns(lifecycle);

create index if not exists detected_patterns_anti_pattern_idx
    on detected_patterns(is_anti_pattern)
 where is_anti_pattern;

create index if not exists detected_patterns_tags_idx
    on detected_patterns using gin(tags);

comment on table detected_patterns is
'Code patterns detected during indexing and analysis.
- lifecycle: suggested (emerging, 2+ places) → gap (recommended but absent) → rule (enforced)
- is_anti_pattern: true for duplication, god-nodes, monoliths, dead code, copy-paste
- fix_pattern_id: for anti-patterns, self-ref to the constructive pattern that would fix it
- evidence: [{session_id, file, line, description}] — sessions/files where pattern was observed
- instances: [{file, line, snippet}] — specific code locations
- family: GoF category (structural, behavioral, creational), resilience, data-access, etc.';

comment on column detected_patterns.id
     is 'Surrogate primary key (UUID).';
comment on column detected_patterns.folder_id
     is 'Foreign key to folders — which repo this pattern was detected in.';
comment on column detected_patterns.name
     is 'Pattern name (e.g. "Adapter", "Duplicated auth guard", "God node · router.ts").';
comment on column detected_patterns.family
     is 'Pattern family: GoF · structural, GoF · behavioral, resilience, data-access, etc.';
comment on column detected_patterns.lifecycle
     is 'Pattern state: suggested (emerging), gap (recommended but absent), rule (enforced in sessions).';
comment on column detected_patterns.is_anti_pattern
     is 'True for anti-patterns (duplication, god-node, monolith, dead-code, copy-paste).';
comment on column detected_patterns.severity
     is 'Anti-pattern severity: low, medium, high. Null for positive patterns.';
comment on column detected_patterns.confidence
     is 'Detection confidence score 0.00-1.00.';
comment on column detected_patterns.instance_count
     is 'Number of code locations where this pattern is detected.';
comment on column detected_patterns.instances
     is 'JSON array: [{file, line, snippet}] — specific code locations.';
comment on column detected_patterns.evidence
     is 'JSON array: [{session_id, file, description}] — sessions/files that confirmed this pattern.';
comment on column detected_patterns.fix_pattern_id
     is 'For anti-patterns: self-ref FK to the constructive pattern that would fix this issue.';
comment on column detected_patterns.description
     is 'Human-readable description of the pattern and why it matters.';
comment on column detected_patterns.example
     is 'Code example showing the pattern in use (or the anti-pattern to avoid).';
comment on column detected_patterns.enforcement
     is 'How this pattern is enforced when lifecycle = rule (e.g. "new auth integrations must adapt, not inline").';
comment on column detected_patterns.tags
     is 'Array of tag strings for filtering.';
comment on column detected_patterns.detected_at
     is 'Timestamp when this pattern was first detected.';
comment on column detected_patterns.modified_at
     is 'Timestamp of the last modification to this row.';
