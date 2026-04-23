-- Scanned roots — folders the user has pointed sensei at.

CREATE TABLE IF NOT EXISTS sensei.scanned_roots (
  path       text        PRIMARY KEY,
  created_at timestamptz NOT NULL DEFAULT now()
);

COMMENT ON TABLE sensei.scanned_roots IS 'Folders sensei has scanned. Watchers run for active roots.';
