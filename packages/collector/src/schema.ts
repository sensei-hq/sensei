import { Database } from "bun:sqlite";

export function createTables(db: Database): void {
  db.run("PRAGMA journal_mode=WAL");

  db.run(`
    CREATE TABLE IF NOT EXISTS events (
      id          INTEGER PRIMARY KEY AUTOINCREMENT,
      user_uuid   TEXT    NOT NULL,
      session_id  TEXT    NOT NULL DEFAULT '',
      seq         INTEGER,
      ts          INTEGER NOT NULL,
      tool        TEXT    NOT NULL,
      phase       TEXT    NOT NULL CHECK(phase IN ('pre', 'post')),
      duration_ms INTEGER,
      success     INTEGER,
      input       TEXT,
      error       TEXT,
      project_path TEXT   NOT NULL DEFAULT ''
    )
  `);

  db.run(`
    CREATE TABLE IF NOT EXISTS projects (
      path       TEXT PRIMARY KEY,
      first_seen INTEGER NOT NULL,
      last_seen  INTEGER NOT NULL
    )
  `);

  db.run(`
    CREATE TABLE IF NOT EXISTS daily_stats (
      date             TEXT NOT NULL,
      tool             TEXT NOT NULL,
      calls            INTEGER NOT NULL DEFAULT 0,
      successes        INTEGER NOT NULL DEFAULT 0,
      total_duration_ms INTEGER NOT NULL DEFAULT 0,
      PRIMARY KEY (date, tool)
    )
  `);

  db.run("CREATE INDEX IF NOT EXISTS idx_events_session ON events(session_id)");
  db.run("CREATE INDEX IF NOT EXISTS idx_events_tool ON events(tool)");
  db.run("CREATE INDEX IF NOT EXISTS idx_events_ts ON events(ts)");
}
