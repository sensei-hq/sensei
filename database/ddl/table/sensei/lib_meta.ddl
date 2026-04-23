-- Library metadata — external libraries indexed by sensei.

CREATE TABLE IF NOT EXISTS sensei.lib_meta (
  name        text        PRIMARY KEY,
  source_type text        NOT NULL,
  base_url    text,
  used_by     text[]      NOT NULL DEFAULT '{}',
  indexed_at  timestamptz
);

COMMENT ON TABLE sensei.lib_meta IS 'External libraries that sensei wraps (no MCP of their own).';
