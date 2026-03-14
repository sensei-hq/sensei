// packages/engine/src/session/snapshot.ts
import type { SupabaseClient } from "@supabase/supabase-js";

export interface SnapshotOptions {
  kind: "manual" | "checkpoint";
  progressSummary: string;
  nextStepHint?: string;
  completedSteps?: string[];
  inFlightFiles?: string[];
  worktreeRefs?: Array<{ branch: string; path: string; status: string }>;
  diffStatSummary?: string;
}

export interface Snapshot {
  id: string;
  kind: "manual" | "checkpoint";
  progressSummary: string;
  nextStepHint: string | null;
  completedSteps: string[];
  inFlightFiles: string[];
  worktreeRefs: Array<{ branch: string; path: string; status: string }>;
  diffStatSummary: string | null;
  createdAt: string;
}

function shapeSnapshot(row: Record<string, unknown>): Snapshot {
  return {
    id: row.id as string,
    kind: row.kind as "manual" | "checkpoint",
    progressSummary: row.progress_summary as string,
    nextStepHint: (row.next_step_hint as string | null) ?? null,
    completedSteps: (row.completed_steps as string[]) ?? [],
    inFlightFiles: (row.in_flight_files as string[]) ?? [],
    worktreeRefs: (row.worktree_refs as Array<{ branch: string; path: string; status: string }>) ?? [],
    diffStatSummary: (row.diff_stat_summary as string | null) ?? null,
    createdAt: row.created_at as string,
  };
}

export async function takeSnapshot(
  db: SupabaseClient,
  sessionId: string,
  repoId: string,
  opts: SnapshotOptions,
): Promise<Snapshot> {
  const { data, error } = await db
    .from("snapshots")
    .insert({
      session_id: sessionId,
      repo_id: repoId,
      kind: opts.kind,
      progress_summary: opts.progressSummary,
      next_step_hint: opts.nextStepHint ?? null,
      completed_steps: opts.completedSteps ?? [],
      in_flight_files: opts.inFlightFiles ?? [],
      worktree_refs: opts.worktreeRefs ?? [],
      diff_stat_summary: opts.diffStatSummary ?? null,
    })
    .select()
    .single();
  if (error || !data) throw new Error(error?.message ?? "Failed to save snapshot");
  return shapeSnapshot(data as Record<string, unknown>);
}

export async function getLatestSnapshot(db: SupabaseClient, sessionId: string): Promise<Snapshot | null> {
  const { data } = await db
    .from("snapshots")
    .select("*")
    .eq("session_id", sessionId)
    .order("created_at", { ascending: false })
    .limit(1)
    .single();
  if (!data) return null;
  return shapeSnapshot(data as Record<string, unknown>);
}
