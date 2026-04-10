# task-manager — Architecture

## Overview
A TypeScript task management library. Three layers: types → utils → services → api.

## File Map
```
src/types.ts              — All shared interfaces: Task, CreateTaskInput, UpdateTaskInput, TaskFilter, TaskSummary
src/utils/validation.ts   — Input validation functions; throws ValidationError (extends Error) with a `field` property
src/utils/date.ts         — Pure date helpers: isPastDue, isDueSoon, formatDueDate, startOfDay, endOfDay
src/services/taskService.ts — TaskService class: in-memory Map<string,Task> store, full CRUD + query methods
src/api/routes.ts         — Framework-agnostic HTTP handlers (ApiRequest/ApiResponse interfaces)
tests/taskService.spec.ts — Vitest tests for TaskService
```

## Key Types
```typescript
type TaskStatus = "todo" | "in-progress" | "done" | "cancelled"
type Priority   = 1 | 2 | 3   // 1 = high, 2 = medium, 3 = low

interface Task {
  id: string; title: string; description?: string;
  status: TaskStatus; priority: Priority;
  tags: string[]; assignee?: string;
  dueDate?: Date; createdAt: Date; updatedAt: Date;
}
```

## TaskService API
```typescript
// CRUD
createTask(input: CreateTaskInput): Task
getTask(id: string): Task | undefined
updateTask(id: string, input: UpdateTaskInput): Task   // throws if not found
deleteTask(id: string): boolean

// Query
getAllTasks(): Task[]
filterTasks(filter: TaskFilter): Task[]              // main query method
getTasksByStatus(status: TaskStatus): Task[]
getTasksByAssignee(assignee: string): Task[]
getTasksByPriority(priority: Priority): Task[]
getOverdueTasks(): Task[]

// Analytics
getSummary(): TaskSummary
```

## Internal Storage
`private tasks = new Map<string, Task>()` — swap for async DB adapter by replacing filter/get/set calls.
IDs generated with `randomUUID()` from Node's `crypto` module.
