// packages/server/src/tools/close-memory.ts
import { getActivityLog } from "../activity-log.js";

interface CloseMemoryParams {
  id: string;
  resolution: string;
}

export async function closeMemoryTool(
  repoId: string,
  params: CloseMemoryParams,
): Promise<{ id: string; type: string; title: string; status: string; resolution: string; closedAt: string }> {
  const log = getActivityLog(repoId);
  const item = log.getBacklogById(params.id);
  const closedAt = new Date().toISOString();

  if (!item) {
    // Graceful response when item not found locally
    return {
      id: params.id,
      type: "question",
      title: "(unknown)",
      status: "done",
      resolution: params.resolution,
      closedAt,
    };
  }

  log.updateBacklogItem(params.id, { status: "done", description: params.resolution });

  return {
    id: item.id,
    type: "question",
    title: item.title,
    status: "done",
    resolution: params.resolution,
    closedAt,
  };
}
