// packages/server/src/tools/take-snapshot.ts
import { getActivityLog } from "../activity-log.js";

interface TakeSnapshotParams {
  progress_summary: string;
  next_step_hint?: string;
  in_flight_files?: string[];
  completed_steps?: string[];
  worktree_refs?: Array<{ branch: string; path: string; status: string }>;
  diff_stat_summary?: string;
}

export async function takeSnapshotTool(
  repoId: string,
  sessionId: string,
  localSessionId: string | undefined,
  params: TakeSnapshotParams,
): Promise<{ id: string; progressSummary: string; createdAt: string }> {
  const log = getActivityLog(repoId);
  const effectiveSessionId = localSessionId ?? sessionId;
  const createdAt = new Date().toISOString();
  const id = log.logSnapshot({
    sessionId: effectiveSessionId,
    repoId,
    progressSummary: params.progress_summary,
    nextStepHint: params.next_step_hint,
    inFlightFiles: params.in_flight_files,
    completedSteps: params.completed_steps,
    diffStatSummary: params.diff_stat_summary,
  });
  return {
    id,
    progressSummary: params.progress_summary,
    createdAt,
  };
}
