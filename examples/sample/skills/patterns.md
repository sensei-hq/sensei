# project-board — Patterns & Conventions

## Error Handling
```typescript
// Use helpers from src/utils/errors.ts — never return { error } manually
notFound("Task", id)           // throws AppError(404, "Task '<id>' not found")
badRequest("Validation failed", issues)  // throws AppError(400, ...)
throw new AppError(501, "Not implemented")

// Global handler registered in index.ts:
app.onError(errorHandler)      // converts AppError → JSON response
```

## Validation Pattern
```typescript
// Middleware approach for body validation:
app.post('/tasks', validate(CreateTaskSchema), (c) => {
  const body = c.get('body') as typeof CreateTaskSchema._type;
  // body is already parsed and typed
});

// Query param parsing (throws 400 on invalid):
const { cursor, limit } = parseQuery(c, PaginationSchema);
const { status, q } = c.req.query();   // raw strings for optional filters
```

## Pagination Pattern
```typescript
// All list endpoints use cursor-based pagination on created_at DESC:
const since = decodeCursor(cursor);      // null if no cursor
const rows = since
  ? db.query('SELECT * FROM x WHERE created_at < ? ORDER BY created_at DESC LIMIT ?').all(since, limit + 1)
  : db.query('SELECT * FROM x ORDER BY created_at DESC LIMIT ?').all(limit + 1);
return c.json(toPage(rows, limit));      // sets nextCursor if more pages exist
```

## DB Access Pattern
```typescript
// Always call getDb() — never new Database() in route files
const db = getDb();

// Parameterised queries (never string interpolation):
db.query<RowType, [string]>('SELECT * FROM x WHERE id = ?').get(id);
db.query<RowType, [string, number]>('SELECT * FROM x WHERE ... LIMIT ?').all(param, limit);
db.run('UPDATE x SET col = ? WHERE id = ?', [value, id]);

// Transactions for multi-step writes:
const doWork = db.transaction((args) => { /* ... */ });
doWork(myArgs);
```

## Immutable Updates Pattern
```typescript
// Use COALESCE to only update fields that are explicitly provided:
db.run(
  'UPDATE tasks SET title = COALESCE(?, title), status = COALESCE(?, status), updated_at = ? WHERE id = ?',
  [body.title ?? null, body.status ?? null, now, id]
);
// Pass null for fields not in the PATCH body — COALESCE keeps the existing value.
```

## FTS5 Search Pattern
```typescript
// tasks_fts is synced automatically via triggers — just query it:
const matching = db.query<{ rowid: number }, [string]>(
  'SELECT rowid FROM tasks_fts WHERE tasks_fts MATCH ? ORDER BY rank'
).all(queryString);
// Then join rowids back to the tasks table for the full record.
```

## Response Shapes
- **List**: `{ data: T[], nextCursor: string | null }` — always use `toPage()`
- **Single**: the entity object directly
- **Created**: entity with status 201
- **Deleted**: 204 no body — `c.body(null, 204)`
- **Bulk**: custom shape per endpoint (documented in feature task file)

## Test Utilities
```typescript
// Standard test helper (copy from existing test files):
async function req(method, path, body?) {
  return app.request(path, {
    method,
    headers: body ? { 'Content-Type': 'application/json' } : {},
    body: body ? JSON.stringify(body) : undefined,
  });
}
// Call resetDb() in beforeEach — gives a fresh in-memory DB each test
```
