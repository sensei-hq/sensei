import { Database } from "bun:sqlite";

let _db: Database | null = null;

/**
 * Returns the singleton DB connection.
 * Path defaults to `DB_PATH` env var, then `:memory:` (for tests).
 * Call `resetDb()` in tests to get a fresh schema each time.
 */
export function getDb(): Database {
  if (!_db) {
    _db = new Database(process.env.DB_PATH ?? ":memory:", { create: true });
    _db.run("PRAGMA journal_mode=WAL");
    _db.run("PRAGMA foreign_keys=ON");
    migrate(_db);
  }
  return _db;
}

/** Drop and recreate the DB — use in tests only. */
export function resetDb(): void {
  _db?.close();
  _db = null;
}

function migrate(db: Database): void {
  db.run(`
    CREATE TABLE IF NOT EXISTS projects (
      id          TEXT PRIMARY KEY,
      name        TEXT NOT NULL,
      description TEXT,
      status      TEXT NOT NULL DEFAULT 'active',
      created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
      updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    )
  `);

  db.run(`
    CREATE TABLE IF NOT EXISTS tasks (
      id          TEXT PRIMARY KEY,
      project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
      title       TEXT NOT NULL,
      description TEXT,
      status      TEXT NOT NULL DEFAULT 'todo',
      priority    INTEGER NOT NULL DEFAULT 2,
      assignee    TEXT,
      due_date    TEXT,
      created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
      updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    )
  `);

  db.run(`
    CREATE TABLE IF NOT EXISTS comments (
      id         TEXT PRIMARY KEY,
      task_id    TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
      author     TEXT NOT NULL,
      body       TEXT NOT NULL,
      created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
      updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    )
  `);

  // FTS5 index for task search (feature 2)
  db.run(`
    CREATE VIRTUAL TABLE IF NOT EXISTS tasks_fts
    USING fts5(title, description, content=tasks, content_rowid=rowid)
  `);

  // Keep FTS index in sync via triggers
  db.run(`
    CREATE TRIGGER IF NOT EXISTS tasks_fts_insert AFTER INSERT ON tasks BEGIN
      INSERT INTO tasks_fts(rowid, title, description) VALUES (new.rowid, new.title, new.description);
    END
  `);
  db.run(`
    CREATE TRIGGER IF NOT EXISTS tasks_fts_update AFTER UPDATE ON tasks BEGIN
      INSERT INTO tasks_fts(tasks_fts, rowid, title, description) VALUES ('delete', old.rowid, old.title, old.description);
      INSERT INTO tasks_fts(rowid, title, description) VALUES (new.rowid, new.title, new.description);
    END
  `);
  db.run(`
    CREATE TRIGGER IF NOT EXISTS tasks_fts_delete AFTER DELETE ON tasks BEGIN
      INSERT INTO tasks_fts(tasks_fts, rowid, title, description) VALUES ('delete', old.rowid, old.title, old.description);
    END
  `);
}
