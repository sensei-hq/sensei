// packages/engine/src/session/session-manager.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import { getLatestSnapshot } from "./snapshot.js";
import type { Snapshot } from "./snapshot.js";

export interface SessionInfo {
  id: string;
  createdAt: string;
}

export interface CrashedSession {
  id: string;
  createdAt: string;
  lastHeartbeat: string;  // last_heartbeat — surfaced as crashedAt in get_session_context
  latestSnapshot: Snapshot | null;
}

export async function createSession(db: SupabaseClient, repoId: string): Promise<SessionInfo> {
  const { data, error } = await db
    .from("sessions")
    .insert({ repo_id: repoId })
    .select("id,created_at")
    .single();
  if (error || !data) throw new Error(error?.message ?? "Failed to create session");
  return { id: data.id as string, createdAt: data.created_at as string };
}

export async function detectCrashedSessions(
  db: SupabaseClient,
  repoId: string,
  idleThresholdMs = 10 * 60 * 1000,
): Promise<CrashedSession[]> {
  try {
    const cutoff = new Date(Date.now() - idleThresholdMs).toISOString();
    const { data, error } = await db
      .from("sessions")
      .select("id,created_at,last_heartbeat")
      .eq("repo_id", repoId)
      .eq("status", "active")
      .lt("last_heartbeat", cutoff)
      .order("created_at", { ascending: false })
      .limit(10);

    if (error || !data || data.length === 0) return [];

    // Mark all as crashed
    await db
      .from("sessions")
      .update({ status: "crashed" })
      .in("id", (data as Array<{ id: string }>).map(s => s.id));

    // Fetch latest snapshot for each
    const results: CrashedSession[] = await Promise.all(
      (data as Array<{ id: string; created_at: string; last_heartbeat: string }>).map(async s => ({
        id: s.id,
        createdAt: s.created_at,
        lastHeartbeat: s.last_heartbeat,
        latestSnapshot: await getLatestSnapshot(db, s.id),
      }))
    );
    return results;
  } catch {
    return [];
  }
}

export async function updateHeartbeat(db: SupabaseClient, sessionId: string): Promise<void> {
  try {
    await db
      .from("sessions")
      .update({ last_heartbeat: new Date().toISOString() })
      .eq("id", sessionId);
  } catch {
    // Silent best-effort
  }
}
