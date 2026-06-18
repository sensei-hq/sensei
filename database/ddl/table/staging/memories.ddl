set search_path to staging, extensions;

drop table if exists memories cascade;
create table memories (
  project_name          text
, namespace_scope_key   text
, namespace_slug        text
, scope                 text
, scope_filter          text
, enforcement           text
, origin                text
, type                  text
, title                 text
, content               text
, impact                text
, strength              real
, status                text
, reinforced_count      integer
, violated_count        integer
, last_relevant_at      timestamptz
, tags                  text[]
, triage_signal         text
, modified_at           timestamptz
);

comment on table staging.memories is
'Staging buffer for sensei.memories. FK targets are carried as natural keys
(project_name; namespace_scope_key + namespace_slug) and resolved to ids during
import. Enum columns are text here (cast during import). Loaded from
import/staging/memories.jsonl (produced by export.memories) via dbd import, then
CALL staging.import_memories() to move into sensei.memories.';
