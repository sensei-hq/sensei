import type { SupabaseClient } from "@supabase/supabase-js";

export async function recordPatternUse(
  client: SupabaseClient,
  repoId: string,
  sessionId: string | null,
  patternName: string,
): Promise<string> {
  const { error } = await (client as any)
    .schema("sensei")
    .from("pattern_usages")
    .insert({ repo_id: repoId, session_id: sessionId, pattern_name: patternName });

  if (error) throw new Error(error.message ?? "Failed to record pattern use");
  return `Pattern use recorded: ${patternName}`;
}
