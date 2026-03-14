// packages/server/src/tools/close-memory.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import { closeMemory as engineCloseMemory } from "@sensei/engine";

interface CloseMemoryParams {
  id: string;
  resolution: string;
}

export async function closeMemoryTool(
  client: SupabaseClient,
  params: CloseMemoryParams,
) {
  const item = await engineCloseMemory(client, params.id, params.resolution);
  return {
    id: item.id,
    type: item.type,
    title: item.title,
    status: item.status,
    resolution: item.resolution,
    closedAt: item.closedAt,
  };
}
