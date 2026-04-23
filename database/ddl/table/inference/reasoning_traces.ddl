set search_path to inference, sensei, extensions;

create table if not exists reasoning_traces (
  id                       uuid        primary key default gen_random_uuid()
, project_id               uuid        references sensei.projects(id) on delete set null
, trigger_event            text        not null
, trigger_detail           jsonb       not null default '{}'
, models_used              text[]      not null default '{}'
, exchanges                jsonb       not null default '[]'
, consensus                jsonb       not null default '{}'
, action_proposed          jsonb
);

create index if not exists reasoning_traces_project_id_idx
    on reasoning_traces(project_id)
 where project_id is not null;

comment on table reasoning_traces is
'MOE consensus panel reasoning traces. Stores the full debate between local models.
- trigger_event: what initiated the reasoning (ftr_drop, recurring_correction, pattern_emerging, etc.)
- models_used: ["gemma3:27b", "qwen3:14b", ...]
- exchanges: [{model, role:"proposer"|"challenger"|"synthesizer", content, tokens}]
- consensus: {conclusion, confidence, disagreements:[]}
- action_proposed: the recommendation generated, if any';

comment on column reasoning_traces.id
     is 'Surrogate primary key (UUID).';
comment on column reasoning_traces.project_id
     is 'Foreign key to projects — which project this reasoning was about. Nullable for system-level.';
comment on column reasoning_traces.trigger_event
     is 'What triggered this reasoning: ftr_drop, recurring_correction, pattern_emerging, negative_impact, etc.';
comment on column reasoning_traces.trigger_detail
     is 'Context for the trigger: {ftr_delta, sessions, module, ...}.';
comment on column reasoning_traces.models_used
     is 'Array of model identifiers used in the reasoning panel.';
comment on column reasoning_traces.exchanges
     is 'Full debate: [{model, role, content, tokens}] in order.';
comment on column reasoning_traces.consensus
     is 'Final consensus: {conclusion, confidence:0.0-1.0, disagreements:[]}.';
comment on column reasoning_traces.action_proposed
     is 'Recommendation generated from this reasoning, if any.';
