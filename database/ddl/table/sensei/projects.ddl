-- Projects — groups of 1+ repos that evolve together.
-- A project is a product/system. Repos within a project share lifecycle.

CREATE TABLE IF NOT EXISTS sensei.projects (
  id          text        PRIMARY KEY,
  name        text        NOT NULL,
  description text,
  client      text,
  category    text        NOT NULL DEFAULT 'active',
  created_at  timestamptz NOT NULL DEFAULT now(),
  updated_at  timestamptz NOT NULL DEFAULT now()
);

COMMENT ON TABLE sensei.projects IS 'A project groups repos that form a product. Default is 1 repo per project.';
COMMENT ON COLUMN sensei.projects.category IS 'active, side, idea, archived.';
