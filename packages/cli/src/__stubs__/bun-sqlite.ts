/**
 * Stub for bun:sqlite used in Vitest test runs for @sensei/collector.
 * Uses better-sqlite3 as the underlying engine so tests run real SQL.
 */
import BetterSqlite3 from "better-sqlite3";

export class Database {
  private _db: BetterSqlite3.Database;

  constructor(path: string) {
    this._db = new BetterSqlite3(path);
  }

  run(sql: string, params?: unknown[]): void {
    this._db.prepare(sql).run(...(params ?? []));
  }

  prepare(sql: string) {
    const stmt = this._db.prepare(sql);
    return {
      all: (...args: unknown[]) => stmt.all(...args),
      get: (...args: unknown[]) => stmt.get(...args),
      run: (...args: unknown[]) => stmt.run(...args),
    };
  }

  close(): void {
    this._db.close();
  }
}
