import type { SupabaseClient } from "@supabase/supabase-js";

export async function tokenStats(client: SupabaseClient, sessionId: string) {
  const { data, error } = await client
    .from("context_packs")
    .select("id,task,total_tokens,created_at")
    .eq("session_id", sessionId)
    .order("created_at", { ascending: false });

  if (error || !data) return { totalPacks: 0, totalTokensServed: 0, avgPackSize: 0, packs: [] };

  const packs = data as Array<{ id: string; task: string; total_tokens: number; created_at: string }>;
  const totalTokensServed = packs.reduce((sum, p) => sum + p.total_tokens, 0);

  return {
    totalPacks: packs.length,
    totalTokensServed,
    avgPackSize: packs.length > 0 ? Math.round(totalTokensServed / packs.length) : 0,
    packs: packs.map(p => ({ id: p.id, task: p.task, totalTokens: p.total_tokens, createdAt: p.created_at })),
  };
}
