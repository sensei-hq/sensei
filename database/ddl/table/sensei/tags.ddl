-- Tags — flexible labels on any entity (repo, project, session).

CREATE TABLE IF NOT EXISTS sensei.tags (
  entity_type text NOT NULL,
  entity_id   text NOT NULL,
  tag         text NOT NULL,
  PRIMARY KEY (entity_type, entity_id, tag)
);

CREATE INDEX IF NOT EXISTS idx_tags_entity ON sensei.tags(entity_type, entity_id);

COMMENT ON TABLE sensei.tags IS 'Generic tag system. entity_type = repo|project|session.';
