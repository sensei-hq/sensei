set search_path to inference, extensions;

create type if not exists preference_confidence
    as enum ('low', 'medium', 'high');

create table if not exists preferences (
  id                       uuid                    primary key default gen_random_uuid()
, project_id               uuid                    references sensei.projects(id) on delete cascade
, category                 text                    not null
, rule                     text                    not null
, source                   text                    not null
, confidence               preference_confidence   not null default 'low'
, correction_count         integer                 not null default 0
, props                    jsonb                   not null default '{}'
, modified_at              timestamptz             not null default now()
);

create index if not exists preferences_project_id_idx
    on preferences(project_id)
 where project_id is not null;

create index if not exists preferences_category_idx
    on preferences(category);

comment on table preferences is
'Learned coding style and workflow preferences.
- project_id: null = global (personal), set = project-specific
- source: how this preference was learned — learned:s-{id}, codebase-scan, explicit
- confidence: low (1 correction), medium (2 corrections or codebase signal), high (3+ corrections or explicit)
- correction_count: how many times user corrected for this preference
- category: naming, file_organization, indentation, code_style, communication, workflow

Included in get_session_context() based on confidence level and matching context.';

comment on column preferences.id
     is 'Surrogate primary key (UUID).';
comment on column preferences.project_id
     is 'Foreign key to projects — null for global personal preferences, set for project-specific.';
comment on column preferences.category
     is 'Preference category: naming, file_organization, indentation, code_style, communication, workflow.';
comment on column preferences.rule
     is 'The preference rule (e.g. "DDL column types align at position 27").';
comment on column preferences.source
     is 'How learned: learned:s-{session_id}, codebase-scan, explicit.';
comment on column preferences.confidence
     is 'Confidence: low (1 signal), medium (2+ signals), high (3+ or explicit declaration).';
comment on column preferences.correction_count
     is 'Number of times user corrected for this preference.';
comment on column preferences.props
     is 'Extensible: {session_ids:[], files_observed:[], last_correction_at, ...}.';
comment on column preferences.modified_at
     is 'Timestamp of the last modification to this row.';
