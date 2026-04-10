import { Hono } from "hono";
import { randomUUID } from "crypto";
import { getDb } from "../db.js";
import type { Project } from "../types.js";
import { CreateProjectSchema, UpdateProjectSchema, PaginationSchema } from "../types.js";
import { validate, parseQuery } from "../utils/validate.js";
import { decodeCursor, encodeCursor, toPage } from "../utils/pagination.js";
import { notFound } from "../utils/errors.js";

export const projects = new Hono();

// GET /projects
projects.get("/", (c) => {
  const { cursor, limit } = parseQuery(c, PaginationSchema);
  const db = getDb();
  const since = decodeCursor(cursor);

  const rows = since
    ? db.query<Project, [string, number]>(
        "SELECT * FROM projects WHERE created_at < ? ORDER BY created_at DESC LIMIT ?",
      ).all(since, limit + 1)
    : db.query<Project, [number]>(
        "SELECT * FROM projects ORDER BY created_at DESC LIMIT ?",
      ).all(limit + 1);

  return c.json(toPage(rows, limit));
});

// POST /projects
projects.post("/", validate(CreateProjectSchema), (c) => {
  const body = c.get("body") as typeof CreateProjectSchema._type;
  const db = getDb();
  const now = new Date().toISOString();
  const id = randomUUID();

  db.run(
    "INSERT INTO projects (id, name, description, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
    [id, body.name, body.description ?? null, now, now],
  );

  const project = db.query<Project, [string]>("SELECT * FROM projects WHERE id = ?").get(id)!;
  return c.json(project, 201);
});

// GET /projects/:id
projects.get("/:id", (c) => {
  const project = db().query<Project, [string]>("SELECT * FROM projects WHERE id = ?").get(c.req.param("id"));
  if (!project) notFound("Project", c.req.param("id"));
  return c.json(project);
});

// PATCH /projects/:id
projects.patch("/:id", validate(UpdateProjectSchema), (c) => {
  const db = getDb();
  const id = c.req.param("id");
  const existing = db.query<Project, [string]>("SELECT * FROM projects WHERE id = ?").get(id);
  if (!existing) notFound("Project", id);

  const body = c.get("body") as typeof UpdateProjectSchema._type;
  const now = new Date().toISOString();

  db.run(
    `UPDATE projects SET
       name        = COALESCE(?, name),
       description = COALESCE(?, description),
       status      = COALESCE(?, status),
       updated_at  = ?
     WHERE id = ?`,
    [body.name ?? null, body.description ?? null, body.status ?? null, now, id],
  );

  return c.json(db.query<Project, [string]>("SELECT * FROM projects WHERE id = ?").get(id)!);
});

// DELETE /projects/:id
projects.delete("/:id", (c) => {
  const db = getDb();
  const id = c.req.param("id");
  const existing = db.query<Project, [string]>("SELECT * FROM projects WHERE id = ?").get(id);
  if (!existing) notFound("Project", id);
  db.run("DELETE FROM projects WHERE id = ?", [id]);
  return c.body(null, 204);
});

// GET /projects/:id/tasks  (lists tasks scoped to a project)
projects.get("/:id/tasks", (c) => {
  const db = getDb();
  const projectId = c.req.param("id");
  const project = db.query<Project, [string]>("SELECT * FROM projects WHERE id = ?").get(projectId);
  if (!project) notFound("Project", projectId);

  const { cursor, limit } = parseQuery(c, PaginationSchema);
  const since = decodeCursor(cursor);

  const rows = since
    ? db.query<{ id: string; created_at: string }, [string, string, number]>(
        "SELECT * FROM tasks WHERE project_id = ? AND created_at < ? ORDER BY created_at DESC LIMIT ?",
      ).all(projectId, since, limit + 1)
    : db.query<{ id: string; created_at: string }, [string, number]>(
        "SELECT * FROM tasks WHERE project_id = ? ORDER BY created_at DESC LIMIT ?",
      ).all(projectId, limit + 1);

  return c.json(toPage(rows as { created_at: string; id: string }[], limit));
});

// Helper to avoid repetition in the two GET handlers
function db() { return getDb(); }
