// packages/server/src/tools/record-memory.ts
import { getActivityLog } from "../activity-log.js";

interface RecordMemoryParams {
  type: "decision" | "pattern" | "question";
  title: string;
  content: string;
}

export async function recordMemoryTool(
  repoId: string,
  sessionId: string | null,
  params: RecordMemoryParams,
  localSessionId?: string,
) {
  const log = getActivityLog(repoId);

  if (params.type === "decision") {
    log.logDecision({
      repoId,
      text: params.title,
      context: params.content,
    });
    return {
      id: crypto.randomUUID(),
      type: params.type,
      title: params.title,
      status: "active",
      createdAt: new Date().toISOString(),
    };
  }

  if (params.type === "pattern") {
    log.logDecision({
      repoId,
      text: params.title,
      context: params.content,
      tags: ["pattern"],
    });
    return {
      id: crypto.randomUUID(),
      type: params.type,
      title: params.title,
      status: "active",
      createdAt: new Date().toISOString(),
    };
  }

  // "question" type → backlog
  const id = log.addBacklogItem({
    repoId,
    title: params.title,
    description: params.content,
    status: "open",
    priority: "medium",
  });

  return {
    id,
    type: params.type,
    title: params.title,
    status: "open",
    createdAt: new Date().toISOString(),
  };
}
