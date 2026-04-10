import { z } from "zod";

// ── Project ───────────────────────────────────────────────────────────────────

export const ProjectStatus = z.enum(["active", "archived"]);
export type ProjectStatus = z.infer<typeof ProjectStatus>;

export const CreateProjectSchema = z.object({
  name: z.string().trim().min(1).max(100),
  description: z.string().max(500).optional(),
});

export const UpdateProjectSchema = z.object({
  name: z.string().trim().min(1).max(100).optional(),
  description: z.string().max(500).optional(),
  status: ProjectStatus.optional(),
});

export type Project = {
  id: string;
  name: string;
  description: string | null;
  status: ProjectStatus;
  created_at: string;
  updated_at: string;
};

// ── Task ──────────────────────────────────────────────────────────────────────

export const TaskStatus = z.enum(["todo", "in-progress", "in-review", "done"]);
export type TaskStatus = z.infer<typeof TaskStatus>;

export const TaskPriority = z.number().int().min(1).max(3);

export const CreateTaskSchema = z.object({
  project_id: z.string().uuid(),
  title: z.string().trim().min(1).max(200),
  description: z.string().max(2000).optional(),
  status: TaskStatus.default("todo"),
  priority: TaskPriority.default(2),
  assignee: z.string().max(100).optional(),
  due_date: z.string().datetime().optional(),
});

export const UpdateTaskSchema = z.object({
  title: z.string().trim().min(1).max(200).optional(),
  description: z.string().max(2000).optional(),
  status: TaskStatus.optional(),
  priority: TaskPriority.optional(),
  assignee: z.string().max(100).optional(),
  due_date: z.string().datetime().optional(),
});

export const BulkUpdateTaskSchema = z.object({
  ids: z.array(z.string().uuid()).min(1).max(100),
  status: TaskStatus,
});

export type Task = {
  id: string;
  project_id: string;
  title: string;
  description: string | null;
  status: TaskStatus;
  priority: number;
  assignee: string | null;
  due_date: string | null;
  created_at: string;
  updated_at: string;
};

// ── Comment ───────────────────────────────────────────────────────────────────

export const CreateCommentSchema = z.object({
  author: z.string().trim().min(1).max(100),
  body: z.string().trim().min(1).max(2000),
});

export type Comment = {
  id: string;
  task_id: string;
  author: string;
  body: string;
  created_at: string;
  updated_at: string;
};

// ── Shared ────────────────────────────────────────────────────────────────────

export const PaginationSchema = z.object({
  cursor: z.string().optional(),
  limit: z.coerce.number().int().min(1).max(100).default(20),
});
