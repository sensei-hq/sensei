import { describe, it, expect } from "vitest";
import { createAccountRepository } from "./account-repository.js";

type Row = Record<string, unknown>;

function makeDb(rows: Row[], schema = "sensei") {
  const calls: string[] = [];
  const mockQ = (table: string) => ({
    select: () => ({
      eq: (col: string, val: unknown) => {
        const filtered1 = rows.filter(r => r[col] === val);
        return {
          // single eq → order → range (list())
          order: () => ({ range: () => Promise.resolve({ data: filtered1, error: null }) }),
          // double eq (get, update, delete)
          eq: (col2: string, val2: unknown) => {
            const filtered2 = filtered1.filter(r => r[col2] === val2);
            return {
              order: () => ({ range: () => Promise.resolve({ data: filtered2, error: null }) }),
              single: () => {
                const found = filtered2[0] ?? null;
                return Promise.resolve({ data: found, error: found ? null : new Error("not found") });
              },
            };
          },
          single: () => {
            const found = filtered1[0] ?? null;
            return Promise.resolve({ data: found, error: found ? null : new Error("not found") });
          },
        };
      },
    }),
    insert: (data: Row) => ({
      select: () => ({ single: () => { calls.push(`insert:${table}:${JSON.stringify(data.account_id)}`); return Promise.resolve({ data: { id: "new-id", ...data }, error: null }); } }),
    }),
    update: (patch: Row) => ({
      eq: (col: string, val: unknown) => ({
        eq: (col2: string, val2: unknown) => ({
          select: () => ({ single: () => {
            const found = rows.find(r => r[col] === val && r[col2] === val2);
            return Promise.resolve({ data: found ? { ...found, ...patch } : null, error: null });
          }}),
        }),
      }),
    }),
    delete: () => ({
      eq: (col: string, val: unknown) => ({
        eq: (col2: string, val2: unknown) => {
          calls.push(`delete:${table}:${col}=${val}:${col2}=${val2}`);
          return Promise.resolve({ error: null });
        },
      }),
    }),
  });

  return {
    db: {
      schema: () => ({ from: (table: string) => mockQ(table) }),
    } as any,
    calls,
  };
}

describe("createAccountRepository", () => {
  it("list() only returns rows for the given account", async () => {
    const rows = [
      { id: "s1", account_id: "acme", created_at: "2026-01-01" },
      { id: "s2", account_id: "other", created_at: "2026-01-02" },
    ];
    const { db } = makeDb(rows);
    const repo = createAccountRepository(db, "acme");
    const results = await repo.sessions.list();
    expect(results.every(r => r.account_id === "acme")).toBe(true);
    expect(results.map(r => r.id)).toContain("s1");
    expect(results.map(r => r.id)).not.toContain("s2");
  });

  it("get() returns null when id belongs to a different account", async () => {
    const rows = [{ id: "s2", account_id: "other", created_at: "2026-01-01" }];
    const { db } = makeDb(rows);
    const repo = createAccountRepository(db, "acme");
    const result = await repo.sessions.get("s2");
    expect(result).toBeNull();
  });

  it("create() always injects account_id", async () => {
    const { db, calls } = makeDb([]);
    const repo = createAccountRepository(db, "acme");
    await repo.repos.create({ name: "my-repo" });
    expect(calls.some(c => c.includes("acme"))).toBe(true);
  });

  it("remove() scopes delete to account_id", async () => {
    const { db, calls } = makeDb([{ id: "r1", account_id: "acme" }]);
    const repo = createAccountRepository(db, "acme");
    await repo.repos.remove("r1");
    expect(calls.some(c => c.includes("account_id=acme"))).toBe(true);
  });
});
