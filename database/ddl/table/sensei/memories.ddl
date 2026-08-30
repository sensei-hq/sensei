set search_path to sensei, extensions;

create table if not exists memories (
  id                       uuid          primary key default gen_random_uuid()
, project_id               uuid          references sensei.projects(id) on delete cascade
, scope                    memory_scope  not null default 'project'
, scope_filter             text
-- Governance plane: a memory positioned on a namespace (where it applies) +
-- enforcement (how much authority it carries). namespace_id supersedes the
-- legacy scope/scope_filter/project_id triple for resolution; the old columns
-- are retained during the transition so the flat assemble_context keeps working.
, namespace_id             uuid          references sensei.namespaces(id) on delete set null
, enforcement              enforcement   not null default 'recommended'
, origin                   text          not null default 'learned'
, source_id                uuid
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
-- Spine-slot anchor (design 2026-07-18-memory-anchoring): which doc slot this
-- memory belongs to, for slot-scoped retrieval. `feature` disambiguates scope
-- (null = project-scope; set = docs/features/<feature>/). Both nullable = unanchored.
, spine_slot               spine_slot
, feature                  text
, triage_signal            text
, category                 memory_category
, created_at               timestamptz   not null default now()
, modified_at              timestamptz   not null default now()
-- Ready-to-share lane: sensei's assessment that this memory has been rewritten
-- project-agnostic and is ready to widen up the scope ladder, plus the portable
-- rewrite itself and its invented illustration (both null until generalised).
-- Everything named `generalised_*` is a candidate to leave this machine; `content`
-- above is not. That naming IS the rule — see the column comments.
, generalised              boolean       not null default false
, generalised_content      text
, generalised_example      text
);

create index if not exists memories_project_id_idx
    on memories(project_id, scope, status);

create index if not exists memories_scope_idx
    on memories(scope, scope_filter)
 where status = 'active';
create index if not exists memories_spine_slot_idx
    on memories(project_id, spine_slot)
 where status = 'active';

-- Resolution lookup: active rules for a namespace ordered by authority.
create index if not exists memories_namespace_idx
    on memories(namespace_id, enforcement, status)
 where status in ('active', 'reinforced', 'battle_tested');

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
     is 'When to surface: global (always), project (this project), stack (matching tech), task_type (matching task), module (matching code area). Legacy axis — superseded by namespace_id for governance resolution.';
comment on column memories.namespace_id
     is 'Governance scope instance this rule applies to (organization/project/technology/...). Null = unscoped (general). Resolution gathers rules across a repo''s member namespaces + always-on general/user.';
comment on column memories.enforcement
     is 'Authority axis: advisory < recommended < required < mandatory. mandatory = non-overridable constitution tier (a more specific scope cannot weaken it).';
comment on column memories.origin
     is 'Provenance: learned (knowledge plane), authored (written directly), promoted (elevated from a narrower scope), federated (pulled from a DÅjÅ), dojo (applied from a Dōjō downstream artifact — see sensei.dojo_inbox).';
comment on column memories.source_id
     is 'When origin=promoted/federated, the id of the source memory (or remote record) this was derived from. Null otherwise.';
comment on column memories.scope_filter
     is 'Qualifier for scope: stack name (e.g. "rust"), task type (e.g. "fix"), module path (e.g. "src/api"). Null for global/project scope.';
comment on column memories.type
     is 'Knowledge category: decision (architectural), pattern (code convention), convention (team norm), preference (style rule), continuity (session handoff), question (open issue).';
comment on column memories.title
     is 'Short label used as heading in context output.';
comment on column memories.content
     is 'The rule or learning — full body text surfaced to the agent. LOCAL REFERENCE: verbatim as captured, so it may quote real code, paths, ids and decisions. It is NOT the shareable form. Never sent on the collective/contribute lane: batch_share_items selects generalised_content only, with no fallback to this column — an un-generalised memory is held, not shipped. It is NOT unconditionally local: the separate federation rule-push (federation::push_promoted → POST /rules) sends this column verbatim, ungeneralised and unstripped, for an origin=promoted memory at a shareable namespace whose knowledge source is push-enabled. That is an explicit publish of the raw rule the user promoted, on a different lane with a different consent model — do not read this column as unsendable.';
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
comment on column memories.category
     is 'Quality dimension for the learnings UI anatomy, orthogonal to `type`: correctness, convention, pattern, preference. Null until classified.';
comment on column memories.created_at
     is 'When this memory was first learned/created (stable). Distinct from modified_at, which moves on every reinforcement; the UI shows this as "learned".';
comment on column memories.generalised
     is 'Ready-to-share flag: true once sensei has rewritten this memory into a project-agnostic rule (stored in generalised_content), meaning it is ready to widen up the scope ladder. Set only by an explicit /generalise action; never fabricated.';
comment on column memories.generalised_content
     is 'The project-agnostic rewrite of `content` — identifiers (project/repo/file/service/person names) stripped and restated as a general principle. Null until generalised. SHAREABLE: this, not `content`, is what the upstream contribute path sends.';
comment on column memories.generalised_example
     is 'A SYNTHETIC illustration of generalised_content: the generalise prompt asks the model to invent a situation similar in SHAPE to the original while quoting none of it. Optional (null when the model produced none) and never fabricated by the daemon to fill a gap. "Synthetic" is a GENERATION CONTRACT carried by the prompt, not a checked invariant: nothing compares this text against `content`, so a model that parrots its input can land real names here. What IS enforced before it leaves the machine is the same deterministic identifier strip the body gets — a KNOWN project/repo/person/path token is removed, and residual risk holds the whole artifact. SHAREABLE, and rewritten together with generalised_content so an example never outlives the rule it illustrates.';
