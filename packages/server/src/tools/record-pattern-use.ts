import { getActivityLog } from "../activity-log.js";

export async function recordPatternUse(
  repoId: string,
  sessionId: string | null,
  patternName: string,
): Promise<string> {
  const log = getActivityLog(repoId);
  log.logPatternUse(repoId, sessionId, patternName);
  return `Pattern use recorded: ${patternName}`;
}
