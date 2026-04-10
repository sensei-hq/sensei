# Feature 1: Task Comments

Implement the comments system so users can discuss tasks.

## Endpoints to implement

### GET /tasks/:id/comments
Return a paginated list of comments for a task.

- Use the same cursor-based pagination as other list endpoints (`PaginationSchema`, `toPage`, `decodeCursor`)
- Response shape: `{ data: Comment[], nextCursor: string | null }`
- Return 404 if the task does not exist
- Order by `created_at DESC`

### POST /tasks/:id/comments
Create a new comment on a task.

- Body validated with `CreateCommentSchema` (already in `src/types.ts`)
- Return 201 with the created comment
- Return 404 if the task does not exist
- Generate a UUID for `id`, set both `created_at` and `updated_at`

### DELETE /comments/:id
Delete a comment by its id.

- Return 204 on success
- Return 404 if the comment does not exist

## Where to make changes

- **`src/routes/comments.ts`** — implement the DELETE handler (stub is already there)
- **`src/routes/tasks.ts`** — implement the GET and POST handlers for `/:id/comments` (stubs marked with 501)
- **`src/index.ts`** — the `comments` router is already imported and mounted at `/comments`; no changes needed

## Key patterns to follow

- Validation: use `validate(Schema)` middleware from `src/utils/validate.ts`
- Errors: use `notFound(resource, id)` from `src/utils/errors.ts`
- DB: call `getDb()` to get the singleton connection — never create a new `Database` directly
- The `comments` table schema is in `src/db.ts`

## Tests

Run `bun test tests/comments.test.ts` to check your implementation.
All tests in that file should pass when done.
