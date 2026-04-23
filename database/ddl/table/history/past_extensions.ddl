set search_path to history, sensei, extensions;

create type if not exists dml_operation
    as enum ('INSERT', 'UPDATE', 'DELETE');


create table if not exists past_extensions (
  id                       uuid           primary key default gen_random_uuid()
, extension_id             uuid           not null
, plugin_id                uuid
, kind                     text           not null
, name                     text           not null
, version                  text
, description              text
, content                  text
, props                    jsonb          not null default '{}'
, scope                    text           not null
, enabled                  boolean        not null
, source                   text           not null
, icons                    jsonb          not null default '{}'
, tags                     text[]         not null default '{}'
, operation                dml_operation   not null
, revision                 integer        not null default 0
, recommendation_id        uuid
, effective_from           timestamptz    not null
, effective_to             timestamptz
, changed_at               timestamptz    not null default now()
, changed_by               text
);

create index if not exists past_extensions_extension_id_idx
    on past_extensions(extension_id);

create index if not exists past_extensions_recommendation_idx
    on past_extensions(recommendation_id)
 where recommendation_id is not null;

comment on table past_extensions is
'History table for sensei.extensions — auto-populated by historize_extensions trigger.
Captures every INSERT, UPDATE, and DELETE so that skill/persona/rule changes can be
audited and correlated with FTR impact via recommendation_id.

- extension_id: references sensei.extensions.id with no FK — survives hard deletes
- recommendation_id: which recommendation triggered this change (null for manual edits)
- effective_from/to: when this revision was the live state
- operation: INSERT, UPDATE, or DELETE';

comment on column past_extensions.id
     is 'Surrogate primary key (UUID).';
comment on column past_extensions.extension_id
     is 'References sensei.extensions.id. No FK — survives hard deletes.';
comment on column past_extensions.recommendation_id
     is 'Which recommendation triggered this change. Null for manual edits. Links to inference.recommendations.';
comment on column past_extensions.operation
     is 'DML that produced this entry: INSERT, UPDATE, or DELETE.';
comment on column past_extensions.revision
     is 'Revision number of the extension at this snapshot.';
comment on column past_extensions.effective_from
     is 'When this revision became the live state.';
comment on column past_extensions.effective_to
     is 'When this revision was superseded. Null if still current or if source was deleted.';
comment on column past_extensions.changed_at
     is 'When this history entry was written.';
comment on column past_extensions.changed_by
     is 'Who triggered the change: user, recommendation:{id}, system.';
