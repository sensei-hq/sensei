# Feature 3: Bulk Task Status Update

Implement `PATCH /tasks/bulk` to update the status of multiple tasks in one request.

## Endpoint

`PATCH /tasks/bulk`

**Request body:**
```json
{
  "ids": ["uuid1", "uuid2", "uuid3"],
  "status": "done"
}
```

**Success response (200):**
```json
{
  "updated": 2,
  "notFound": ["uuid-that-doesnt-exist"]
}
```

## Behaviour

- Validate the body with `BulkUpdateTaskSchema` (already in `src/types.ts`):
  - `ids`: 1–100 UUIDs
  - `status`: valid `TaskStatus`
- For each id:
  - If the task exists → update its `status` and `updated_at`, add to `updated` count
  - If it does not exist → add its id to the `notFound` array
- **Do not throw** if some ids are missing — collect them and return the partial result
- Run all updates in a single SQLite transaction for atomicity
- Return 200 with `{ updated: number, notFound: string[] }`

## Where to make changes

Only **`src/routes/tasks.ts`** — the `PATCH /tasks/bulk` stub is already registered
before `/:id` routes (this ordering matters in Hono; do not change it).

## Key patterns to follow

- Use `validate(BulkUpdateTaskSchema)` middleware
- Use `getDb()` for the connection
- Wrap multiple DB writes in a transaction:
  ```typescript
  const db = getDb();
  const updateMany = db.transaction((ids: string[], status: string) => {
    // your logic here
  });
  updateMany(body.ids, body.status);
  ```

## Tests

Run `bun test tests/tasks.test.ts` — the two `PATCH /tasks/bulk` tests should pass.
