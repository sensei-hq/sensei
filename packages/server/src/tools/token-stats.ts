import { getActivityLog } from "../activity-log.js";

export async function tokenStats(repoId: string, sessionId: string) {
  const log = getActivityLog(repoId);
  const packs = log.getContextPacks(sessionId);
  const totalTokensServed = packs.reduce((sum, p) => sum + p.totalTokens, 0);

  return {
    totalPacks: packs.length,
    totalTokensServed,
    avgPackSize: packs.length > 0 ? Math.round(totalTokensServed / packs.length) : 0,
    packs: packs.map(p => ({ id: p.id, task: p.task, totalTokens: p.totalTokens, createdAt: p.createdAt })),
  };
}
