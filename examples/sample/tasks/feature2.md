# Feature 2: Full-Text Task Search

Add full-text search to `GET /tasks` via the `?q=` query parameter.

## Behaviour

`GET /tasks?q=authentication` should return tasks whose `title` or `description`
contains "authentication", ranked by relevance (FTS5 rank).

- Combine with existing filters: `GET /tasks?q=bug&status=todo&project_id=<id>` should work
- If `?q=` is provided but empty string, treat it as no filter (return all)
- Pagination (`cursor`, `limit`) still applies
- Response shape is unchanged: `{ data: Task[], nextCursor: string | null }`

## Where to make changes

Only **`src/routes/tasks.ts`** needs to change — specifically the `GET /tasks` handler.

Currently it throws 501 when `?q=` is present. Replace that block with a real FTS5 query.

## How to use the FTS5 index

The `tasks_fts` virtual table and its sync triggers are already set up in `src/db.ts`.

```sql
-- Get rowids of matching tasks, ordered by rank (best first)
SELECT rowid FROM tasks_fts WHERE tasks_fts MATCH ? ORDER BY rank

-- Then join back to tasks:
SELECT t.* FROM tasks t
INNER JOIN (
  SELECT rowid FROM tasks_fts WHERE tasks_fts MATCH ? ORDER BY rank
) fts ON t.rowid = fts.rowid
WHERE t.status = ?   -- combine with other filters
ORDER BY fts.rank    -- preserve relevance ordering
LIMIT ?
```

Use parameterised queries. FTS5 MATCH syntax: pass the raw query string as-is; SQLite handles tokenisation.

## Key patterns to follow

- Use `getDb()` for the connection
- Use `parseQuery(c, PaginationSchema)` for cursor/limit — already done in the handler
- Preserve the `toPage` call for the response shape
- Do not change any other endpoints

## Tests

Run `bun test tests/tasks.test.ts` — the two `GET /tasks?q=` tests should pass.
