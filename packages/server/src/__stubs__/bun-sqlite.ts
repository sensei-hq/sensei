/**
 * Stub for bun:sqlite used in Vitest (Node.js) test runs.
 * Provides the minimal Database interface used across @sensei/server:
 *   new Database(path)
 *   db.run(sql, params?)
 *   db.query(sql) → Statement with .get(), .all(), .run()
 *   db.close()
 */
class Statement {
  get(_params?: unknown): null { return null; }
  all(_params?: unknown): [] { return []; }
  run(_params?: unknown): void {}
}

export class Database {
  constructor(_path: string) {}
  run(_sql: string, _params?: unknown[]): void {}
  query(_sql: string): Statement { return new Statement(); }
  close(): void {}
}
