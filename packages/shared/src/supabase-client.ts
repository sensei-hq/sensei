import { createClient, type SupabaseClient } from "@supabase/supabase-js";
import { loadSenseiConfig, loadCredentials } from "./config.js";

/** Build a service-role Supabase client scoped to the `sensei` schema. Returns null if config missing. */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function makeSenseiClient(repoPath: string): Promise<SupabaseClient<any, "sensei"> | null> {
  const [config, creds] = await Promise.all([
    loadSenseiConfig(repoPath),
    loadCredentials(),
  ]);
  if (!config || !creds || !config.supabase_url) return null;
  // db.schema scopes all queries to sensei.* — no need for .schema() chaining on each call
  return createClient(config.supabase_url, creds.supabase_service_key, {
    db: { schema: "sensei" as const },
    auth: { persistSession: false },
  }) as SupabaseClient<any, "sensei">;
}
