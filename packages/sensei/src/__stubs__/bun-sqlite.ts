/**
 * Stub for bun:sqlite used in Vitest (Node.js) test runs.
 * Provides the minimal Database interface that serve.ts uses:
 *   new Database(path)
 *   db.run(sql, params?)
 */
export class Database {
  constructor(_path: string) {}
  run(_sql: string, _params?: unknown[]): void {}
}
