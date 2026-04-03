// packages/server/src/auth-middleware.ts
// Platform API auth middleware — verifies Supabase JWT and resolves account context.
import type { SupabaseClient } from "@supabase/supabase-js";

export interface SessionCallerContext {
  accountId: string;
  userId: string;
  role: "platform_admin" | "account_admin" | "user";
}

export interface AuthVerifyResult extends SessionCallerContext {
  email: string;
  accountSlug: string;
  accountType: "individual" | "team" | "platform";
}

/**
 * Verifies a Supabase JWT and resolves the caller's account context.
 * Returns null on auth failure (caller should respond 401/403).
 */
export async function resolveCallerContext(
  db: SupabaseClient,
  authHeader: string | null,
): Promise<AuthVerifyResult | null> {
  if (!authHeader?.startsWith("Bearer ")) return null;
  const token = authHeader.slice(7);

  // Verify JWT via Supabase getUser()
  let userId: string;
  let email: string;
  try {
    const { data, error } = await db.auth.getUser(token);
    if (error || !data?.user) return null;
    userId = data.user.id;
    email = data.user.email ?? "";
  } catch {
    return null;
  }

  // Lookup profile_accounts
  const { data: mapping, error: mapErr } = await (db as any)
    .schema("core")
    .from("profile_accounts")
    .select("account_id, role, accounts(id, slug, account_type)")
    .eq("user_id", userId)
    .limit(1)
    .single();

  if (mapErr || !mapping) return null;

  const account = (mapping as any).accounts as Record<string, string>;

  return {
    userId,
    email,
    accountId: mapping.account_id as string,
    role: mapping.role as "platform_admin" | "account_admin" | "user",
    accountSlug: account.slug ?? "",
    accountType: account.account_type as "individual" | "team" | "platform",
  };
}

/**
 * Bun.serve-compatible auth handler for the GET /auth/verify endpoint.
 */
export async function handleAuthVerify(
  db: SupabaseClient,
  req: Request,
): Promise<Response> {
  const caller = await resolveCallerContext(db, req.headers.get("Authorization"));
  if (!caller) {
    return Response.json({ error: "Unauthorized" }, { status: 401 });
  }
  return Response.json(caller);
}

/**
 * callerFilter — returns the appropriate account/user filter for DB queries.
 *   platform_admin → unrestricted (empty filter)
 *   account_admin  → scoped to account
 *   user           → scoped to account + user
 */
export function callerFilter(caller: SessionCallerContext): Record<string, string> {
  if (caller.role === "platform_admin") return {};
  if (caller.role === "account_admin") return { accountId: caller.accountId };
  return { accountId: caller.accountId, userId: caller.userId };
}
