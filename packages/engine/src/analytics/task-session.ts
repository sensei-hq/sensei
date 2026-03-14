// packages/engine/src/analytics/task-session.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import { computeAndStoreFtr } from "./ftr.js";
import type { FtrResult, FtrSignals } from "./ftr.js";

export type TaskType = 'feat' | 'fix' | 'refactor' | 'docs' | 'test' | 'chore' | 'unknown';
export type TaskStatus = 'in_progress' | 'completed' | 'abandoned';

export interface TaskSessionInfo {
  id: string;
  taskType: TaskType | null;
  createdAt: string;
}

export interface TaskSession {
  id: string;
  sessionId: string | null;
  repoId: string;
  taskDescription: string | null;
  taskType: TaskType | null;
  status: TaskStatus;
  ftrScore: number | null;
  ftrSignals: FtrSignals | null;
  createdAt: string;
  completedAt: string | null;
}

function detectTaskType(description: string): TaskType {
  const lower = description.toLowerCase();
  if (/fix|bug|patch|broken|error/.test(lower)) return 'fix';
  if (/test|spec|coverage/.test(lower)) return 'test';
  if (/feat|add|implement|build|create|new/.test(lower)) return 'feat';
  if (/refactor|clean|extract|restructure/.test(lower)) return 'refactor';
  if (/docs?|document|readme|comment/.test(lower)) return 'docs';
  if (/chore|bump|update|upgrade|dep/.test(lower)) return 'chore';
  return 'unknown';
}

export async function createTaskSession(
  db: SupabaseClient,
  sessionId: string,
  repoId: string,
  taskDescription?: string,
): Promise<TaskSessionInfo> {
  const taskType: TaskType = taskDescription ? detectTaskType(taskDescription) : 'unknown';

  const { data, error } = await db
    .from("task_sessions")
    .insert({
      session_id: sessionId,
      repo_id: repoId,
      task_description: taskDescription ?? null,
      task_type: taskType,
    })
    .select()
    .single();

  if (error || !data) throw new Error((error as { message?: string } | null)?.message ?? "Failed to create task session");

  return {
    id: (data as Record<string, unknown>).id as string,
    taskType: (data as Record<string, unknown>).task_type as TaskType | null,
    createdAt: (data as Record<string, unknown>).created_at as string,
  };
}

export async function recordTaskTurn(
  db: SupabaseClient,
  taskSessionId: string,
  repoId: string,
  tool: string,
  success: boolean | null,
  durationMs?: number | null,
): Promise<void> {
  try {
    await db.from("task_turns").insert({
      task_session_id: taskSessionId,
      repo_id: repoId,
      tool,
      success,
      duration_ms: durationMs ?? null,
    });
  } catch {
    // silent best-effort — a missed turn row widens error rate estimate but never blocks
  }
}
