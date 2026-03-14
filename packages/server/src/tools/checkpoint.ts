// packages/server/src/tools/checkpoint.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import { takeSnapshot } from "@sensei/engine";

interface CheckpointParams {
  task_summary: string;
  completed_steps?: string[];
}

export async function checkpointTool(
  client: SupabaseClient,
  sessionId: string,
  repoId: string,
  params: CheckpointParams,
) {
  // Write checkpoint snapshot
  const snapshot = await takeSnapshot(client, sessionId, repoId, {
    kind: "checkpoint",
    progressSummary: params.task_summary,
    completedSteps: params.completed_steps,
  });

  // Mark session completed
  const { error: updateError } = await client
    .from("sessions")
    .update({ status: "completed" })
    .eq("id", sessionId);
  if (updateError) throw new Error(updateError.message ?? "Failed to mark session completed");

  return {
    id: snapshot.id,
    kind: snapshot.kind,
    progressSummary: snapshot.progressSummary,
    createdAt: snapshot.createdAt,
  };
}
