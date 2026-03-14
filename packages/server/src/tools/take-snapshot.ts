// packages/server/src/tools/take-snapshot.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import { takeSnapshot as engineTakeSnapshot } from "@sensei/engine";

interface TakeSnapshotParams {
  progress_summary: string;
  next_step_hint?: string;
  in_flight_files?: string[];
  completed_steps?: string[];
  worktree_refs?: Array<{ branch: string; path: string; status: string }>;
  diff_stat_summary?: string;
}

export async function takeSnapshotTool(
  client: SupabaseClient,
  sessionId: string,
  repoId: string,
  params: TakeSnapshotParams,
) {
  const snapshot = await engineTakeSnapshot(client, sessionId, repoId, {
    kind: "manual",
    progressSummary: params.progress_summary,
    nextStepHint: params.next_step_hint,
    completedSteps: params.completed_steps,
    inFlightFiles: params.in_flight_files,
    worktreeRefs: params.worktree_refs,
    diffStatSummary: params.diff_stat_summary,
  });
  return {
    id: snapshot.id,
    kind: snapshot.kind,
    progressSummary: snapshot.progressSummary,
    createdAt: snapshot.createdAt,
  };
}
