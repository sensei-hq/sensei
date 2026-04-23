-- Events — session-level activity captured by hooks and MCP tools.
-- Types: turn, revision_requested, tool_used, phase_transition, mindset_applied, etc.

CREATE TABLE IF NOT EXISTS sensei.events (
  id         text        PRIMARY KEY DEFAULT gen_random_uuid()::text,
  project    text        NOT NULL,
  session_id text        REFERENCES sensei.sessions(id),
  event_type text        NOT NULL,
  data       jsonb       NOT NULL DEFAULT '{}',
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_events_project  ON sensei.events(project, created_at);
CREATE INDEX IF NOT EXISTS idx_events_type     ON sensei.events(project, event_type);
CREATE INDEX IF NOT EXISTS idx_events_session  ON sensei.events(session_id);

COMMENT ON TABLE sensei.events IS 'Activity events within sessions. Source for FTR, pattern, and coaching metrics.';
