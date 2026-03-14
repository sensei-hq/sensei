// packages/engine/src/session/memory.ts
import type { SupabaseClient } from "@supabase/supabase-js";

export interface MemoryItem {
  id: string;
  type: "decision" | "pattern" | "question";
  title: string;
  content: string;
  status: "open" | "closed";
  resolution: string | null;
  closedAt: string | null;
  createdAt: string;
}

function shapeMemoryItem(row: Record<string, unknown>): MemoryItem {
  return {
    id: row.id as string,
    type: row.type as "decision" | "pattern" | "question",
    title: row.title as string,
    content: row.content as string,
    status: row.status as "open" | "closed",
    resolution: (row.resolution as string | null) ?? null,
    closedAt: (row.closed_at as string | null) ?? null,
    createdAt: row.created_at as string,
  };
}

export async function recordMemory(
  db: SupabaseClient,
  repoId: string,
  sessionId: string,
  opts: { type: "decision" | "pattern" | "question"; title: string; content: string },
): Promise<MemoryItem> {
  const { data, error } = await db
    .from("memory_items")
    .insert({ repo_id: repoId, session_id: sessionId, type: opts.type, title: opts.title, content: opts.content })
    .select()
    .single();
  if (error || !data) throw new Error(error?.message ?? "Failed to record memory");
  return shapeMemoryItem(data as Record<string, unknown>);
}

export async function closeMemory(
  db: SupabaseClient,
  itemId: string,
  resolution: string,
): Promise<MemoryItem> {
  // Check current status first
  const { data: existing } = await db
    .from("memory_items")
    .select("status")
    .eq("id", itemId)
    .single();
  if (existing && (existing as Record<string, unknown>).status === "closed") {
    throw new Error("Memory item already closed");
  }

  const { data, error } = await db
    .from("memory_items")
    .update({ status: "closed", resolution, closed_at: new Date().toISOString() })
    .eq("id", itemId)
    .select()
    .single();
  if (error || !data) throw new Error(error?.message ?? "Memory item not found");
  return shapeMemoryItem(data as Record<string, unknown>);
}

export async function getMemoryItems(db: SupabaseClient, repoId: string): Promise<MemoryItem[]> {
  try {
    const { data, error } = await db
      .from("memory_items")
      .select("*")
      .eq("repo_id", repoId)
      .order("created_at", { ascending: false });
    if (error || !data) return [];
    return (data as Array<Record<string, unknown>>).map(shapeMemoryItem);
  } catch {
    return [];
  }
}
