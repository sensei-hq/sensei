# project-board — Architecture

## Overview
A TypeScript REST API for project/task management. Built with Hono (HTTP) + bun:sqlite (storage).

## File Map
```
src/
  index.ts          — Hono app, route mounting, global error handler, Bun.serve() entry
  db.ts             — singleton SQLite connection, schema migrations, resetDb() for tests
  types.ts          — Zod schemas + inferred TS types for all entities
  utils/
    errors.ts       — AppError class, notFound() / badRequest() helpers, errorHandler()
    pagination.ts   — cursor-based pagination: decodeCursor, encodeCursor, toPage<T>
    validate.ts     — validate(schema) middleware, parseQuery(c, schema) helper
  routes/
    projects.ts     — /projects CRUD + /projects/:id/tasks list
    tasks.ts        — /tasks CRUD, /tasks/bulk stub, /tasks/:id/comments stubs
    comments.ts     — /comments/:id DELETE stub
tests/
  projects.test.ts  — all passing (complete)
  tasks.test.ts     — CRUD passing; search and bulk tests failing (stubs)
  comments.test.ts  — all failing (not yet implemented)
tasks/
  feature1.md — implement comments (GET/POST /tasks/:id/comments, DELETE /comments/:id)
  feature2.md — implement full-text search via GET /tasks?q=
  feature3.md — implement PATCH /tasks/bulk
```

## Database Schema
```sql
projects (id TEXT PK, name, description, status DEFAULT 'active', created_at, updated_at)
tasks    (id TEXT PK, project_id FK→projects, title, description, status DEFAULT 'todo',
          priority INT DEFAULT 2, assignee, due_date, created_at, updated_at)
comments (id TEXT PK, task_id FK→tasks, author, body, created_at, updated_at)
tasks_fts — FTS5 virtual table synced to tasks via INSERT/UPDATE/DELETE triggers
```

## Route Summary
| Method | Path | Status |
|--------|------|--------|
| GET/POST | /projects | ✓ complete |
| GET/PATCH/DELETE | /projects/:id | ✓ complete |
| GET | /projects/:id/tasks | ✓ complete |
| GET/POST | /tasks | ✓ (GET ?q= is stub) |
| PATCH | /tasks/bulk | stub → feature3.md |
| GET/PATCH/DELETE | /tasks/:id | ✓ complete |
| GET/POST | /tasks/:id/comments | stub → feature1.md |
| DELETE | /comments/:id | stub → feature1.md |

## Key Dependencies
- `hono` — HTTP framework; route handlers use `(c: Context) => Response`
- `bun:sqlite` — built-in SQLite; synchronous API (`db.query().all()`, `db.run()`)
- `zod` — validation; schemas in `src/types.ts`, validated via `validate()` middleware
