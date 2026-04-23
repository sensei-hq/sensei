set search_path to inference, sensei, extensions;

create type if not exists recommendation_urgency
    as enum ('low', 'medium', 'high');

create type if not exists recommendation_status
    as enum ('pending', 'accepted', 'dismissed', 'superseded');

create type if not exists recommendation_verdict
    as enum ('pending', 'positive', 'negative', 'neutral');

create table if not exists recommendations (
  id                       uuid                      primary key default gen_random_uuid()
, project_id               uuid                      not null references sensei.projects(id) on delete cascade
, reasoning_trace_id       uuid                      references inference.reasoning_traces(id) on delete set null
, urgency                  recommendation_urgency    not null default 'medium'
, status                   recommendation_status     not null default 'pending'
, title                    text                      not null
, why                      text                      not null
, impact                   text
, evidence                 jsonb                     not null default '[]'
, action_type              text                      not null
, action_detail            jsonb                     not null default '{}'
, prompt                   text
, default_acp              text
, baseline_ftr             numeric(4,3)
, current_ftr              numeric(4,3)
, verdict                  recommendation_verdict    not null default 'pending'
, props                    jsonb                     not null default '{}'
, acted_at                 timestamptz
, measured_at              timestamptz
);

create index if not exists recommendations_project_id_idx
    on recommendations(project_id, status);

create index if not exists recommendations_urgency_idx
    on recommendations(urgency);

create index if not exists recommendations_verdict_idx
    on recommendations(verdict)
 where verdict != 'pending';

comment on table recommendations is
'The full lifecycle: insight → recommendation → action → measurement.

A recommendation is a point-in-time event. When accepted, it marks the moment
something changed in the system (persona added, pattern promoted, skill enabled).
FTR trend before vs after that point tells us if it helped.

Lifecycle: pending → accepted (user acts) → measured (verdict assigned)
                   → dismissed (user skips)
                   → superseded (newer recommendation replaces)

- baseline_ftr: project FTR at the time recommendation was accepted (acted_at)
- current_ftr: rolling FTR after the change
- verdict: positive (FTR improved), negative (FTR dropped), neutral, pending
- measured_at: when verdict was last computed';

comment on column recommendations.id
     is 'Surrogate primary key (UUID).';
comment on column recommendations.project_id
     is 'Foreign key to projects — which project this targets.';
comment on column recommendations.reasoning_trace_id
     is 'Foreign key to reasoning_traces — the MOE debate that produced this. Nullable for heuristic.';
comment on column recommendations.urgency
     is 'Priority: high (recurring failures), medium (emerging patterns), low (housekeeping).';
comment on column recommendations.status
     is 'Lifecycle: pending → accepted, dismissed, or superseded.';
comment on column recommendations.title
     is 'Short title (e.g. "Write an auth integration-test persona").';
comment on column recommendations.why
     is 'Why this matters, with evidence summary.';
comment on column recommendations.impact
     is 'Projected impact (e.g. "Projected FTR +14%").';
comment on column recommendations.evidence
     is 'JSON array: [{session_id, file, description}] — what triggered this.';
comment on column recommendations.action_type
     is 'What to do: create_persona, promote_pattern, enable_skill, fix_anti_pattern, audit_stale, revise_rule.';
comment on column recommendations.action_detail
     is 'Specifics: {persona_name, pattern_id, skill_id, cwd, ...}.';
comment on column recommendations.prompt
     is 'Ready-to-send prompt for the ACP. Shown in the action drawer.';
comment on column recommendations.default_acp
     is 'Suggested ACP: claude-code, cursor, codex, etc.';
comment on column recommendations.baseline_ftr
     is 'Project FTR snapshot at time of acceptance (acted_at). Null until accepted.';
comment on column recommendations.current_ftr
     is 'Rolling project FTR after the change. Updated during measurement.';
comment on column recommendations.verdict
     is 'Impact: positive (FTR improved), negative (FTR dropped), neutral, pending.';
comment on column recommendations.props
     is 'Extensible: {baseline_corrections_avg, current_corrections_avg, tool_usage_delta, ...}.';
comment on column recommendations.acted_at
     is 'Timestamp when the user accepted — the point-in-time marker for before/after FTR comparison.';
comment on column recommendations.measured_at
     is 'Timestamp when verdict was last computed.';
