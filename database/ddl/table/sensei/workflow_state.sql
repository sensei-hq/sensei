-- Workflow state — current phase/task/issue per project.

CREATE TABLE IF NOT EXISTS sensei.workflow_state (
  project         text        PRIMARY KEY,
  active_phase    text,
  active_plan     text,
  active_task     text,
  active_issue    integer,
  last_checkpoint text,
  rules_hash      text,
  updated_at      timestamptz NOT NULL DEFAULT now()
);

COMMENT ON TABLE sensei.workflow_state IS 'Per-project workflow state. Updated by checkpoint and phase commands.';
