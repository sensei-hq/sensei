import { randomUUID } from "crypto";
import type {
  Task,
  TaskStatus,
  Priority,
  CreateTaskInput,
  UpdateTaskInput,
  TaskFilter,
  TaskSummary,
} from "../types.js";
import { validateCreateInput, validateUpdateInput } from "../utils/validation.js";
import { isPastDue } from "../utils/date.js";

/**
 * In-memory task store. Swap the `tasks` Map for a DB adapter
 * by replacing `find`, `save`, and `remove` with async I/O.
 */
export class TaskService {
  private tasks = new Map<string, Task>();

  // ── CRUD ──────────────────────────────────────────────────────────────────

  createTask(input: CreateTaskInput): Task {
    const validated = validateCreateInput(input);
    const now = new Date();
    const task: Task = {
      id: randomUUID(),
      title: validated.title,
      description: validated.description,
      status: "todo",
      priority: validated.priority,
      tags: validated.tags,
      assignee: validated.assignee,
      dueDate: validated.dueDate ? new Date(validated.dueDate) : undefined,
      createdAt: now,
      updatedAt: now,
    };
    this.tasks.set(task.id, task);
    return task;
  }

  getTask(id: string): Task | undefined {
    return this.tasks.get(id);
  }

  updateTask(id: string, input: UpdateTaskInput): Task {
    const existing = this.tasks.get(id);
    if (!existing) throw new Error(`Task ${id} not found`);

    const validated = validateUpdateInput(input);
    const updated: Task = {
      ...existing,
      ...validated,
      dueDate: validated.dueDate !== undefined ? new Date(validated.dueDate) : existing.dueDate,
      updatedAt: new Date(),
    };
    this.tasks.set(id, updated);
    return updated;
  }

  deleteTask(id: string): boolean {
    return this.tasks.delete(id);
  }

  // ── Querying ───────────────────────────────────────────────────────────────

  getAllTasks(): Task[] {
    return [...this.tasks.values()];
  }

  filterTasks(filter: TaskFilter): Task[] {
    return [...this.tasks.values()].filter(task => {
      if (filter.status && task.status !== filter.status) return false;
      if (filter.priority && task.priority !== filter.priority) return false;
      if (filter.assignee && task.assignee !== filter.assignee) return false;
      if (filter.tag && !task.tags.includes(filter.tag)) return false;
      if (filter.dueBefore && task.dueDate && task.dueDate > filter.dueBefore) return false;
      if (filter.dueAfter && task.dueDate && task.dueDate < filter.dueAfter) return false;
      return true;
    });
  }

  getTasksByStatus(status: TaskStatus): Task[] {
    return [...this.tasks.values()].filter(t => t.status === status);
  }

  getTasksByAssignee(assignee: string): Task[] {
    return [...this.tasks.values()].filter(t => t.assignee === assignee);
  }

  getTasksByPriority(priority: Priority): Task[] {
    return [...this.tasks.values()].filter(t => t.priority === priority);
  }

  getOverdueTasks(): Task[] {
    const now = new Date();
    return [...this.tasks.values()].filter(
      t => t.dueDate && isPastDue(t.dueDate, now) && t.status !== "done" && t.status !== "cancelled",
    );
  }

  // ── Analytics ─────────────────────────────────────────────────────────────

  getSummary(): TaskSummary {
    const all = this.getAllTasks();
    const byStatus = { todo: 0, "in-progress": 0, done: 0, cancelled: 0 } as Record<TaskStatus, number>;
    const byPriority = { 1: 0, 2: 0, 3: 0 } as Record<Priority, number>;

    for (const t of all) {
      byStatus[t.status]++;
      byPriority[t.priority]++;
    }

    return {
      total: all.length,
      byStatus,
      byPriority,
      overdue: this.getOverdueTasks().length,
    };
  }
}
