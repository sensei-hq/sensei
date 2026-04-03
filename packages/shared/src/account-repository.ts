// packages/shared/src/account-repository.ts
// Tenant-safe repository factory — account_id is baked in at construction.
import type { SupabaseClient } from "@supabase/supabase-js";

// ─── Scoped entity operations ──────────────────────────────────────────────

export interface ScopedRepository<T> {
  list(opts?: { limit?: number; offset?: number }): Promise<T[]>;
  get(id: string): Promise<T | null>;
  create(data: Omit<T, "id" | "account_id" | "created_at">): Promise<T>;
  update(id: string, data: Partial<Omit<T, "id" | "account_id" | "created_at">>): Promise<T | null>;
  remove(id: string): Promise<void>;
}

// ─── AccountRepository ─────────────────────────────────────────────────────

export interface AccountRepository {
  repos:         ScopedRepository<Record<string, unknown>>;
  sessions:      ScopedRepository<Record<string, unknown>>;
  taskSessions:  ScopedRepository<Record<string, unknown>>;
  snapshots:     ScopedRepository<Record<string, unknown>>;
  memories:      ScopedRepository<Record<string, unknown>>;
  patternUsages: ScopedRepository<Record<string, unknown>>;
  apiRequests:   ScopedRepository<Record<string, unknown>>;
}

// ─── AdminRepository (no account scoping) ──────────────────────────────────

export interface AdminRepository {
  accounts: {
    list(): Promise<Record<string, unknown>[]>;
    get(id: string): Promise<Record<string, unknown> | null>;
  };
  accountKeys: {
    get(accountId: string): Promise<Record<string, unknown> | null>;
    upsert(accountId: string, wrappedDek: Uint8Array, kekVersion?: number): Promise<void>;
  };
}

// ─── Internal scoped builder ────────────────────────────────────────────────

function makeScopedRepo<T extends Record<string, unknown>>(
  db: SupabaseClient,
  table: string,
  accountId: string,
  schema = "sensei",
): ScopedRepository<T> {
  const q = () => (db as any).schema(schema).from(table);

  return {
    async list({ limit = 100, offset = 0 } = {}) {
      const { data, error } = await q()
        .select("*")
        .eq("account_id", accountId)
        .order("created_at", { ascending: false })
        .range(offset, offset + limit - 1);
      if (error) throw new Error(error.message);
      return (data ?? []) as T[];
    },

    async get(id: string) {
      const { data, error } = await q()
        .select("*")
        .eq("id", id)
        .eq("account_id", accountId)
        .single();
      if (error || !data) return null;
      return data as T;
    },

    async create(row) {
      const { data, error } = await q()
        .insert({ ...row, account_id: accountId })
        .select()
        .single();
      if (error || !data) throw new Error(error?.message ?? "Insert failed");
      return data as T;
    },

    async update(id: string, patch) {
      const { data, error } = await q()
        .update(patch)
        .eq("id", id)
        .eq("account_id", accountId)
        .select()
        .single();
      if (error || !data) return null;
      return data as T;
    },

    async remove(id: string) {
      const { error } = await q()
        .delete()
        .eq("id", id)
        .eq("account_id", accountId);
      if (error) throw new Error(error.message);
    },
  };
}

// ─── Factory ───────────────────────────────────────────────────────────────

export function createAccountRepository(db: SupabaseClient, accountId: string): AccountRepository {
  return {
    repos:         makeScopedRepo(db, "repos",          accountId),
    sessions:      makeScopedRepo(db, "sessions",       accountId),
    taskSessions:  makeScopedRepo(db, "task_sessions",  accountId),
    snapshots:     makeScopedRepo(db, "snapshots",      accountId),
    memories:      makeScopedRepo(db, "memories",       accountId),
    patternUsages: makeScopedRepo(db, "pattern_usages", accountId),
    apiRequests:   makeScopedRepo(db, "api_requests",   accountId),
  };
}

export function createAdminRepository(db: SupabaseClient): AdminRepository {
  return {
    accounts: {
      async list() {
        const { data, error } = await (db as any).schema("core").from("accounts").select("*");
        if (error) throw new Error(error.message);
        return data ?? [];
      },
      async get(id: string) {
        const { data, error } = await (db as any).schema("core").from("accounts").select("*").eq("id", id).single();
        if (error || !data) return null;
        return data;
      },
    },
    accountKeys: {
      async get(accountId: string) {
        const { data } = await (db as any).schema("core").from("account_keys").select("*").eq("account_id", accountId).single();
        return data ?? null;
      },
      async upsert(accountId: string, wrappedDek: Uint8Array, kekVersion = 1) {
        await (db as any).schema("core").from("account_keys").upsert({
          account_id: accountId,
          wrapped_dek: Array.from(wrappedDek),
          kek_version: kekVersion,
        }, { onConflict: "account_id" });
      },
    },
  };
}
