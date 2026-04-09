// packages/server/src/tools/checkpoint.ts
import { getActivityLog } from "../activity-log.js";

interface CheckpointParams {
  task_summary: string;
  completed_steps?: string[];
}

export async function checkpointTool(
  sessionId: string,
  repoId: string,
  params: CheckpointParams,
  repoPath?: string,
  localSessionId?: string,
): Promise<{ id: string; kind: string; progressSummary: string; createdAt: string }> {
  const log = getActivityLog(repoId);

  // Update session in ActivityLog
  if (localSessionId) {
    log.updateSession(localSessionId, {
      completedAt: new Date().toISOString(),
      outcome: "completed",
      summary: params.task_summary,
    });
  }

  // Log checkpoint snapshot to ActivityLog
  const snapshotId = log.logSnapshot({
    sessionId: localSessionId ?? sessionId,
    repoId,
    progressSummary: params.task_summary,
    completedSteps: params.completed_steps,
  });

  return {
    id: snapshotId,
    kind: "checkpoint",
    progressSummary: params.task_summary,
    createdAt: new Date().toISOString(),
  };
}
