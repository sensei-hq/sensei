/**
 * HTTP route handlers (framework-agnostic request/response shapes).
 * Wire these to Express, Hono, or any HTTP framework.
 */

import type { CreateTaskInput, UpdateTaskInput, TaskFilter, TaskStatus, Priority } from "../types.js";
import { TaskService } from "../services/taskService.js";
import { ValidationError } from "../utils/validation.js";

export interface ApiRequest {
  params: Record<string, string>;
  query: Record<string, string>;
  body: unknown;
}

export interface ApiResponse {
  status: number;
  body: unknown;
}

const service = new TaskService();

function ok(body: unknown): ApiResponse {
  return { status: 200, body };
}

function created(body: unknown): ApiResponse {
  return { status: 201, body };
}

function notFound(id: string): ApiResponse {
  return { status: 404, body: { error: `Task ${id} not found` } };
}

function badRequest(message: string): ApiResponse {
  return { status: 400, body: { error: message } };
}

// GET /tasks
export function listTasks(req: ApiRequest): ApiResponse {
  const filter: TaskFilter = {};
  if (req.query.status) filter.status = req.query.status as TaskStatus;
  if (req.query.priority) filter.priority = parseInt(req.query.priority) as Priority;
  if (req.query.assignee) filter.assignee = req.query.assignee;
  if (req.query.tag) filter.tag = req.query.tag;
  if (req.query.dueBefore) filter.dueBefore = new Date(req.query.dueBefore);

  const tasks = Object.keys(filter).length > 0
    ? service.filterTasks(filter)
    : service.getAllTasks();

  return ok(tasks);
}

// POST /tasks
export function createTask(req: ApiRequest): ApiResponse {
  try {
    const task = service.createTask(req.body as CreateTaskInput);
    return created(task);
  } catch (err) {
    if (err instanceof ValidationError) return badRequest(err.message);
    throw err;
  }
}

// GET /tasks/:id
export function getTask(req: ApiRequest): ApiResponse {
  const task = service.getTask(req.params.id);
  return task ? ok(task) : notFound(req.params.id);
}

// PATCH /tasks/:id
export function updateTask(req: ApiRequest): ApiResponse {
  try {
    const task = service.updateTask(req.params.id, req.body as UpdateTaskInput);
    return ok(task);
  } catch (err) {
    if (err instanceof ValidationError) return badRequest(err.message);
    if (err instanceof Error && err.message.includes("not found")) return notFound(req.params.id);
    throw err;
  }
}

// DELETE /tasks/:id
export function deleteTask(req: ApiRequest): ApiResponse {
  const deleted = service.deleteTask(req.params.id);
  return deleted ? ok({ deleted: true }) : notFound(req.params.id);
}

// GET /tasks/summary
export function getTaskSummary(_req: ApiRequest): ApiResponse {
  return ok(service.getSummary());
}
