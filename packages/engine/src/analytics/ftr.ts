import type { SupabaseClient } from "@supabase/supabase-js";

export interface FtrSignals {
  snapshotCount: number;
  toolErrorRate: number;     // 0.0–1.0
  completedCleanly: boolean;
  hasDescription: boolean;
}

export interface FtrResult {
  score: number;             // 0.000–1.000
  signals: FtrSignals;
}

export function computeFtr(signals: FtrSignals): number {
  let score = 1.0;

  // Snapshot penalty: -0.05 per snapshot beyond the first, capped at -0.30
  const extraSnapshots = Math.max(0, signals.snapshotCount - 1);
  score -= Math.min(extraSnapshots * 0.05, 0.30);

  // Tool error rate penalty
  if (signals.toolErrorRate >= 0.20) {
    score -= 0.20;
  } else if (signals.toolErrorRate >= 0.10) {
    score -= 0.10;
  }

  // Session completion penalty
  if (!signals.completedCleanly) {
    score -= 0.30;
  }

  // No description cap (applied after all penalties)
  if (!signals.hasDescription) {
    score = Math.min(score, 0.70);
  }

  // Clamp to [0.0, 1.0] and round to 3 decimal places
  return Math.round(Math.max(0, Math.min(1, score)) * 1000) / 1000;
}

export async function computeAndStoreFtr(
  db: SupabaseClient,
  taskSessionId: string,
  sessionId: string,
): Promise<FtrResult> {
  // 1. Snapshot count (all kinds: manual + checkpoint)
  const { count: snapshotCount, error: snapErr } = await db
    .from("snapshots")
    .select("*", { count: "exact", head: true })
    .eq("session_id", sessionId);
  if (snapErr) throw new Error((snapErr as { message?: string }).message ?? "Failed to fetch snapshot count");

  // 2. Task turns error rate
  const { data: turns, error: turnsErr } = await db
    .from("task_turns")
    .select("success")
    .eq("task_session_id", taskSessionId);
  if (turnsErr) throw new Error((turnsErr as { message?: string }).message ?? "Failed to fetch task turns");
  const allTurns = (turns ?? []) as Array<{ success: boolean | null }>;
  const totalTurns = allTurns.length;
  const errorCount = allTurns.filter(t => t.success === false).length;
  // Zero turns: no errors recorded → toolErrorRate = 0 (best-case default)
  const toolErrorRate = totalTurns > 0 ? errorCount / totalTurns : 0;

  // 3. Session completion status; completedCleanly = status === 'completed'
  //    (checkpointTool sets status='completed' before this is called on the clean path)
  const { data: session, error: sessErr } = await db
    .from("sessions")
    .select("status")
    .eq("id", sessionId)
    .single();
  if (sessErr) throw new Error((sessErr as { message?: string }).message ?? "Failed to fetch session status");

  // 4. Task description presence
  const { data: taskSession, error: tsErr } = await db
    .from("task_sessions")
    .select("task_description")
    .eq("id", taskSessionId)
    .single();
  if (tsErr) throw new Error((tsErr as { message?: string }).message ?? "Failed to fetch task session");

  const signals: FtrSignals = {
    snapshotCount: snapshotCount ?? 0,
    toolErrorRate,
    completedCleanly: (session as Record<string, unknown> | null)?.status === "completed",
    hasDescription: !!(taskSession as Record<string, unknown> | null)?.task_description,
  };

  const score = computeFtr(signals);

  const { error } = await db
    .from("task_sessions")
    .update({ ftr_score: score, ftr_signals: signals })
    .eq("id", taskSessionId);

  if (error) throw new Error((error as { message?: string }).message ?? "Failed to store FTR");

  return { score, signals };
}
