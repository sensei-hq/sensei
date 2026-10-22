set search_path to export, sensei, extensions;

-- Exportable projection of sensei.memories for `dbd export -n export.memories`.
-- Resolves FK uuids to lookup-able natural keys so import can regenerate them:
--   workspace_id -> workspaces.name
--   namespace_id -> namespaces.(scope_key, slug)
-- Drops the surrogate id, session_id (provenance, not enforced FK), and source_id
-- (self-ref lineage for promoted/federated memories — no natural key, and unused
-- by current data which is all origin='learned'). Enum columns cast to text.
-- Column set matches staging.memories for round-trip via import_memories().
create or replace view memories as
select p.name              as workspace_name
     , n.scope_key         as namespace_scope_key
     , n.slug              as namespace_slug
     , m.scope::text       as scope
     , m.scope_filter
     , m.enforcement::text as enforcement
     , m.origin
     , m.type::text        as type
     , m.title
     , m.content
     , m.impact
     , m.strength
     , m.status::text      as status
     , m.reinforced_count
     , m.violated_count
     , m.last_relevant_at
     , m.tags
     , m.triage_signal
     , m.modified_at
from sensei.memories m
left join sensei.workspaces p on p.id = m.workspace_id
left join sensei.namespaces n on n.id = m.namespace_id;

comment on view export.memories is
'Exportable projection of sensei.memories with FK uuids resolved to natural keys (workspace name; namespace scope_key+slug). Drops id/source_id/session_id; casts enums to text. Columns match staging.memories for round-trip via import_memories().';
