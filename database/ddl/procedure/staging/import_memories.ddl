set search_path to staging, sensei, extensions;

-- ── Import procedure ────────────────────────────────────────────────────────
--
-- Transforms rows in staging.memories into sensei.memories, regenerating FK ids
-- from the natural keys carried in staging:
--   project_name                      -> sensei.projects.id
--   (namespace_scope_key, namespace_slug) -> sensei.namespaces.id
-- Scalar subqueries (LIMIT 1) are used rather than joins so a staging row maps to
-- exactly one output row even when a natural key is non-unique (projects.name has
-- duplicates from the junk-project bug #60). Unresolved keys yield NULL (a global /
-- unscoped memory). Enum columns are cast from text here.
--
-- Mirrors import_hook_events: a plain insert intended for a freshly-reset/empty
-- target (the reset → apply → import workflow). id/source_id/session_id are not
-- carried; ids regenerate, source_id/session_id stay NULL.
--
-- Usage:
--   dbd import memories            -- loads import/staging/memories.jsonl into staging.memories
--   CALL staging.import_memories();-- staging -> sensei.memories (FK regen + enum cast)
--   TRUNCATE staging.memories;
create or replace procedure staging.import_memories()
language plpgsql
as $$
declare
  v_count int := 0;
begin
  insert into sensei.memories
    (project_id, namespace_id, scope, scope_filter, enforcement, origin, type,
     title, content, impact, strength, status, reinforced_count, violated_count,
     last_relevant_at, tags, triage_signal, modified_at)
  select
      (select p.id from sensei.projects p where p.name = stg.project_name limit 1)
    , (select n.id from sensei.namespaces n
         where n.scope_key = stg.namespace_scope_key
           and n.slug      = stg.namespace_slug
         limit 1)
    , coalesce(stg.scope, 'project')::sensei.memory_scope
    , stg.scope_filter
    , coalesce(stg.enforcement, 'recommended')::sensei.enforcement
    , coalesce(stg.origin, 'learned')
    , stg.type::sensei.memory_type
    , stg.title
    , stg.content
    , stg.impact
    , coalesce(stg.strength, 1.0)
    , coalesce(stg.status, 'active')::sensei.memory_status
    , coalesce(stg.reinforced_count, 0)
    , coalesce(stg.violated_count, 0)
    , stg.last_relevant_at
    , coalesce(stg.tags, '{}')
    , stg.triage_signal
    , coalesce(stg.modified_at, now())
  from staging.memories stg
  where stg.title is not null
    and stg.content is not null
    and stg.type is not null
  ;

  get diagnostics v_count = row_count;

  raise notice 'import_memories: inserted % rows into sensei.memories', v_count;
end;
$$;

comment on procedure staging.import_memories() is
'Import staging.memories into sensei.memories. Regenerates FK ids from natural keys (project_name; namespace_scope_key+slug) via scalar lookups; casts enum columns from text. Plain insert for the reset→apply→import workflow (mirrors import_hook_events).';
