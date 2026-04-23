-- Library documentation sections — indexed content from external libs.

CREATE TABLE IF NOT EXISTS sensei.lib_docs (
  id          text        PRIMARY KEY,
  lib_name    text        NOT NULL REFERENCES sensei.lib_meta(name),
  title       text        NOT NULL,
  url         text,
  local_path  text,
  summary     text        NOT NULL DEFAULT '',
  content     text,
  source_type text        NOT NULL,
  component   text,
  indexed_at  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_lib_docs_lib ON sensei.lib_docs(lib_name);

COMMENT ON TABLE sensei.lib_docs IS 'Indexed documentation sections for external libraries.';
