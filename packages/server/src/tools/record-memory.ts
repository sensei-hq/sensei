// packages/server/src/tools/record-memory.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import { recordMemory as engineRecordMemory } from "@sensei/engine";

interface RecordMemoryParams {
  type: "decision" | "pattern" | "question";
  title: string;
  content: string;
}

export async function recordMemoryTool(
  client: SupabaseClient,
  repoId: string,
  sessionId: string,
  params: RecordMemoryParams,
) {
  const item = await engineRecordMemory(client, repoId, sessionId, params);
  return {
    id: item.id,
    type: item.type,
    title: item.title,
    status: item.status,
    createdAt: item.createdAt,
  };
}
