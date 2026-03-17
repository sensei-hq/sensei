// packages/server/src/tools/checkpoint.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import { takeSnapshot } from "@sensei/engine";
import { exec } from "child_process";
import { promisify } from "util";

const execAsync = promisify(exec);

interface CheckpointParams {
  task_summary: string;
  completed_steps?: string[];
}

/** Get files changed in the last commit. Approximation — deferred until session_start_commit is stored. */
async function getChangedFiles(repoPath: string): Promise<string[]> {
  try {
    const { stdout } = await execAsync("git diff --name-only HEAD~1..HEAD", { cwd: repoPath });
    return stdout
      .split("\n")
      .map((f) => f.trim())
      .filter(Boolean);
  } catch {
    return [];
  }
}

export async function checkpointTool(
  client: SupabaseClient,
  sessionId: string,
  repoId: string,
  params: CheckpointParams,
  repoPath?: string,
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

  // Close open pattern_usages rows for this session
  const { data: openRows, error: selectError } = await (client as any)
    .schema("sensei")
    .from("pattern_usages")
    .select("id")
    .eq("session_id", sessionId)
    .is("outcome", null);

  if (!selectError && openRows && openRows.length > 0) {
    const ids = openRows.map((r: { id: string }) => r.id);
    const filesModified = repoPath ? await getChangedFiles(repoPath) : [];

    // ftr_score propagation is deferred — requires engine to expose score
    await (client as any)
      .schema("sensei")
      .from("pattern_usages")
      .update({ outcome: "completed", files_modified: filesModified })
      .eq("session_id", sessionId)
      .in("id", ids);
  }

  return {
    id: snapshot.id,
    kind: snapshot.kind,
    progressSummary: snapshot.progressSummary,
    createdAt: snapshot.createdAt,
  };
}
