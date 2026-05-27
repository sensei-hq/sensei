set search_path to sensei, extensions;

create table if not exists memories (
  id                       uuid          primary key default gen_random_uuid()
, project_id               uuid          references sensei.projects(id) on delete cascade
, scope                    memory_scope  not null default 'project'
, scope_filter             text
, type                     memory_type   not null
, title                    text          not null
, content                  text          not null
, impact                   text
, strength                 real          not null default 1.0
, status                   memory_status not null default 'active'
, reinforced_count         integer       not null default 0
, violated_count           integer       not null default 0
, last_relevant_at         timestamptz
, session_id               uuid
, tags                     text[]        not null default '{}'
, triage_signal            text
, modified_at              timestamptz   not null default now()
);

create index if not exists memories_project_id_idx
    on memories(project_id, scope, status);

create index if not exists memories_scope_idx
    on memories(scope, scope_filter)
 where status = 'active';

create index if not exists memories_strength_idx
    on memories(strength desc)
 where status = 'active';

create index if not exists memories_tags_idx
    on memories using gin (tags);

comment on table memories is
'Multi-level, reasoned, evolving knowledge system.
Replaces memory_items (project-scoped only) and inference.preferences (style rules).
- scope: determines when this memory is surfaced — global, project, stack, task_type, module
- scope_filter: qualifier for non-global scopes (stack name, task type, module path)
- impact: consequence of ignoring this memory — the "why"
- strength: 0–5 score, reinforced by evidence, decayed by time
- session_id: session that created this memory (provenance)

Context assembly: SELECT active memories matching scope hierarchy for current session.';

comment on column memories.id
     is 'Surrogate primary key (UUID).';
comment on column memories.project_id
     is 'Foreign key to projects. Null = global memory (applies to all projects).';
comment on column memories.scope
     is 'When to surface: global (always), project (this project), stack (matching tech), task_type (matching task), module (matching code area).';
comment on column memories.scope_filter
     is 'Qualifier for scope: stack name (e.g. "rust"), task type (e.g. "fix"), module path (e.g. "src/api"). Null for global/project scope.';
comment on column memories.type
     is 'Knowledge category: decision (architectural), pattern (code convention), convention (team norm), preference (style rule), continuity (session handoff), question (open issue).';
comment on column memories.title
     is 'Short label used as heading in context output.';
comment on column memories.content
     is 'The rule or learning — full body text surfaced to the agent.';
comment on column memories.impact
     is 'Consequence of ignoring this memory. Answers "what breaks if you skip this?"';
comment on column memories.strength
     is 'Confidence score 0–5. Created at 1.0, reinforced +1.0, confirmed = 5.0, decayed over time. Below 1.0 = auto-archived.';
comment on column memories.status
     is 'Lifecycle: active (newly learned), reinforced (evidence accumulated), challenged (violated recently), battle_tested (high strength + zero violations over time), archived (retained for history, not surfaced).';
comment on column memories.reinforced_count
     is 'Number of times evidence has confirmed this memory.';
comment on column memories.violated_count
     is 'Number of times this memory was violated (assistant acted contrary to it).';
comment on column memories.last_relevant_at
     is 'Timestamp of last reinforcement or violation. Used for recency-based surfacing.';
comment on column memories.session_id
     is 'Session that created this memory. Null for imported or collective memories.';
comment on column memories.modified_at
     is 'Timestamp of the last modification to this row.';
comment on column memories.tags
     is 'Free-form tags (e.g. security, performance, compliance). GIN-indexed for &&/@> filters.';
comment on column memories.triage_signal
     is 'Which capture heuristic surfaced this memory (revert/correction/actually/repeat_pattern/override/test_failure). Null for explicit /save.';
