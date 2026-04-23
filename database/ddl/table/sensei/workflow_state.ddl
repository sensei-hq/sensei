set search_path to sensei, extensions;

create table if not exists workflow_state (
  project         text        primary key
, active_phase   text
, active_plan   text
, active_task   text
, active_issue  integer
, last_checkpoint text
, rules_hash    text
, updated_at    timestamptz not null default now()
);

comment on table workflow_state is
'Workflow state — current phase/task/issue per project.
Updated by checkpoint and phase commands.';

comment on column workflow_state.project is 'Project identifier.';
comment on column workflow_state.active_phase is 'Current phase name.';
comment on column workflow_state.active_plan is 'Current plan or goal.';
comment on column workflow_state.active_task is 'Current task description.';
comment on column workflow_state.active_issue is 'Current issue number being addressed.';
comment on column workflow_state.last_checkpoint is 'Timestamp of last checkpoint command.';
comment on column workflow_state.rules_hash is 'Hash of active rules for this project.';
comment on column workflow_state.updated_at is 'Timestamp of last update.';