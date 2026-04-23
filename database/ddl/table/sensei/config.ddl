-- Config — key-value user preferences.

CREATE TABLE IF NOT EXISTS sensei.config (
  key   text PRIMARY KEY,
  value text NOT NULL
);

COMMENT ON TABLE sensei.config IS 'User preferences. setup_complete, active_project, sidebar_max_items, etc.';
