export type TaskStatus = "todo" | "in-progress" | "done" | "cancelled";
export type Priority = 1 | 2 | 3; // 1 = high, 2 = medium, 3 = low

export interface Task {
  id: string;
  title: string;
  description?: string;
  status: TaskStatus;
  priority: Priority;
  tags: string[];
  assignee?: string;
  dueDate?: Date;
  createdAt: Date;
  updatedAt: Date;
}

export interface CreateTaskInput {
  title: string;
  description?: string;
  priority?: Priority;
  tags?: string[];
  assignee?: string;
  dueDate?: string; // ISO date string
}

export interface UpdateTaskInput {
  title?: string;
  description?: string;
  status?: TaskStatus;
  priority?: Priority;
  tags?: string[];
  assignee?: string;
  dueDate?: string;
}

export interface TaskFilter {
  status?: TaskStatus;
  priority?: Priority;
  assignee?: string;
  tag?: string;
  dueBefore?: Date;
  dueAfter?: Date;
}

export interface TaskSummary {
  total: number;
  byStatus: Record<TaskStatus, number>;
  byPriority: Record<Priority, number>;
  overdue: number;
}
