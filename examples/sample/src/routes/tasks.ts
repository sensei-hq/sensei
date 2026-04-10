import { Hono } from "hono";
import { randomUUID } from "crypto";
import { getDb } from "../db.js";
import type { Task } from "../types.js";
import {
  CreateTaskSchema,
  UpdateTaskSchema,
  BulkUpdateTaskSchema,
  PaginationSchema,
} from "../types.js";
import { validate, parseQuery } from "../utils/validate.js";
import { decodeCursor, toPage } from "../utils/pagination.js";
import { notFound, AppError } from "../utils/errors.js";

export const tasks = new Hono();

// ── Complete endpoints ────────────────────────────────────────────────────────

// GET /tasks
tasks.get("/", (c) => {
  const { cursor, limit } = parseQuery(c, PaginationSchema);
  const { status, priority, assignee, project_id } = c.req.query();
  const db = getDb();
  const since = decodeCursor(cursor);

  // NOTE: full-text search via ?q= is not yet implemented (see tasks/feature2.md)
  const q = c.req.query("q");
  if (q) throw new AppError(501, "Full-text search not yet implemented — see tasks/feature2.md");

  const conditions: string[] = [];
  const params: (string | number)[] = [];

  if (since) { conditions.push("created_at < ?"); params.push(since); }
  if (status) { conditions.push("status = ?"); params.push(status); }
  if (priority) { conditions.push("priority = ?"); params.push(Number(priority)); }
  if (assignee) { conditions.push("assignee = ?"); params.push(assignee); }
  if (project_id) { conditions.push("project_id = ?"); params.push(project_id); }

  const where = conditions.length ? `WHERE ${conditions.join(" AND ")}` : "";
  params.push(limit + 1);

  const rows = db
    .query<Task, typeof params>(`SELECT * FROM tasks ${where} ORDER BY created_at DESC LIMIT ?`)
    .all(...params);

  return c.json(toPage(rows, limit));
});

// POST /tasks
tasks.post("/", validate(CreateTaskSchema), (c) => {
  const body = c.get("body") as typeof CreateTaskSchema._type;
  const db = getDb();

  // Verify project exists
  const project = db.query("SELECT id FROM projects WHERE id = ?").get(body.project_id);
  if (!project) throw new AppError(422, `Project '${body.project_id}' not found`);

  const now = new Date().toISOString();
  const id = randomUUID();

  db.run(
    `INSERT INTO tasks (id, project_id, title, description, status, priority, assignee, due_date, created_at, updated_at)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
    [id, body.project_id, body.title, body.description ?? null, body.status, body.priority, body.assignee ?? null, body.due_date ?? null, now, now],
  );

  return c.json(db.query<Task, [string]>("SELECT * FROM tasks WHERE id = ?").get(id)!, 201);
});

// ── Bulk endpoint (stub — see tasks/feature3.md) ─────────────────────────────

// IMPORTANT: register /bulk BEFORE /:id so it doesn't get captured as a task id
tasks.patch("/bulk", (_c) => {
  throw new AppError(501, "Bulk task update not yet implemented — see tasks/feature3.md");
});

// ── Single task endpoints ─────────────────────────────────────────────────────

// GET /tasks/:id
tasks.get("/:id", (c) => {
  const task = getDb().query<Task, [string]>("SELECT * FROM tasks WHERE id = ?").get(c.req.param("id"));
  if (!task) notFound("Task", c.req.param("id"));
  return c.json(task);
});

// PATCH /tasks/:id
tasks.patch("/:id", validate(UpdateTaskSchema), (c) => {
  const db = getDb();
  const id = c.req.param("id");
  if (!db.query("SELECT id FROM tasks WHERE id = ?").get(id)) notFound("Task", id);

  const body = c.get("body") as typeof UpdateTaskSchema._type;
  const now = new Date().toISOString();

  db.run(
    `UPDATE tasks SET
       title       = COALESCE(?, title),
       description = COALESCE(?, description),
       status      = COALESCE(?, status),
       priority    = COALESCE(?, priority),
       assignee    = COALESCE(?, assignee),
       due_date    = COALESCE(?, due_date),
       updated_at  = ?
     WHERE id = ?`,
    [body.title ?? null, body.description ?? null, body.status ?? null, body.priority ?? null, body.assignee ?? null, body.due_date ?? null, now, id],
  );

  return c.json(db.query<Task, [string]>("SELECT * FROM tasks WHERE id = ?").get(id)!);
});

// DELETE /tasks/:id
tasks.delete("/:id", (c) => {
  const db = getDb();
  const id = c.req.param("id");
  if (!db.query("SELECT id FROM tasks WHERE id = ?").get(id)) notFound("Task", id);
  db.run("DELETE FROM tasks WHERE id = ?", [id]);
  return c.body(null, 204);
});

// ── Comment endpoints (stub — see tasks/feature1.md) ─────────────────────────

tasks.get("/:id/comments", (_c) => {
  throw new AppError(501, "Task comments not yet implemented — see tasks/feature1.md");
});

tasks.post("/:id/comments", (_c) => {
  throw new AppError(501, "Task comments not yet implemented — see tasks/feature1.md");
});
