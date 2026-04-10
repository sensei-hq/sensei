/**
 * Stub for bun:sqlite used in Vitest test runs.
 * Uses better-sqlite3 as the underlying engine so tests run real SQL.
 * Handles bun:sqlite calling convention: params passed as a single array.
 */
import BetterSqlite3 from 'better-sqlite3';

function unwrap(args: unknown[]): unknown[] {
  if (args.length === 1 && Array.isArray(args[0])) return args[0] as unknown[];
  return args;
}

export class Database {
  private _db: BetterSqlite3.Database;

  constructor(path: string) {
    this._db = new BetterSqlite3(path === ':memory:' ? ':memory:' : path);
  }

  exec(sql: string): void {
    this._db.exec(sql);
  }

  run(sql: string, ...params: unknown[]): void {
    this._db.prepare(sql).run(...unwrap(params));
  }

  prepare(sql: string) {
    const stmt = this._db.prepare(sql);
    return {
      all: (...args: unknown[]) => stmt.all(...unwrap(args)),
      get: (...args: unknown[]) => stmt.get(...unwrap(args)),
      run: (...args: unknown[]) => stmt.run(...unwrap(args)),
    };
  }

  transaction<T>(fn: () => T): () => T {
    return this._db.transaction(fn) as () => T;
  }

  close(): void {
    this._db.close();
  }
}
