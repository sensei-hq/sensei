// packages/engine/src/analytics/ftr-coaching.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import type { FtrSignals } from "./ftr.js";

export interface FtrCoachingHint {
  pattern: "high_snapshot_count" | "high_tool_error_rate" | "incomplete_sessions" | "missing_descriptions";
  frequency: number;   // how many of the last N sessions showed this pattern
  hint: string;
}

const LOW_FTR_THRESHOLD = 0.7;
const LOOKBACK_SESSIONS = 10;

export async function detectFtrCoaching(
  db: SupabaseClient,
  repoId: string,
): Promise<FtrCoachingHint[]> {
  try {
    const { data, error } = await db
      .from("task_sessions")
      .select("ftr_score, ftr_signals")
      .eq("repo_id", repoId)
      .eq("status", "completed")
      .not("ftr_score", "is", null)
      .order("created_at", { ascending: false })
      .limit(LOOKBACK_SESSIONS);

    if (error || !data || data.length < 2) return [];

    const rows = data as Array<{ ftr_score: number | null; ftr_signals: FtrSignals | null }>;
    const lowFtrRows = rows.filter(r => (r.ftr_score ?? 1) < LOW_FTR_THRESHOLD);

    if (lowFtrRows.length === 0) return [];

    // Count pattern occurrences
    let highSnapshotCount = 0;
    let highToolErrorRate = 0;
    let incompleteSessions = 0;
    let missingDescriptions = 0;

    for (const row of lowFtrRows) {
      const s = row.ftr_signals;
      if (!s) continue;
      if (s.snapshotCount > 3) highSnapshotCount++;
      if (s.toolErrorRate >= 0.10) highToolErrorRate++;
      if (!s.completedCleanly) incompleteSessions++;
      if (!s.hasDescription) missingDescriptions++;
    }

    const hints: FtrCoachingHint[] = [];

    if (highSnapshotCount >= 2) {
      hints.push({
        pattern: "high_snapshot_count",
        frequency: highSnapshotCount,
        hint: `${highSnapshotCount} of your recent sessions had many interruptions (high snapshot count). Break large tasks into smaller checkpoints upfront.`,
      });
    }

    if (highToolErrorRate >= 2) {
      hints.push({
        pattern: "high_tool_error_rate",
        frequency: highToolErrorRate,
        hint: `${highToolErrorRate} of your recent sessions had a high tool error rate. Run zero-errors-policy checks before and after implementation.`,
      });
    }

    if (incompleteSessions >= 2) {
      hints.push({
        pattern: "incomplete_sessions",
        frequency: incompleteSessions,
        hint: `${incompleteSessions} recent sessions were not closed with checkpoint(). Always call checkpoint() when done — even on blocked tasks.`,
      });
    }

    if (missingDescriptions >= 2) {
      hints.push({
        pattern: "missing_descriptions",
        frequency: missingDescriptions,
        hint: `${missingDescriptions} recent sessions had no task description. Pass task_description to get_session_context() for accurate FTR tracking.`,
      });
    }

    return hints;
  } catch {
    return [];
  }
}
