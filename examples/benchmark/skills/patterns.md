# task-manager — Patterns & Conventions

## Validation Pattern
- All public inputs go through `validateCreateInput` / `validateUpdateInput` from `utils/validation.ts`
- Throw `ValidationError(message, field)` — never return error objects
- `validateTags` normalises to lowercase + deduplicates (using `Set`)
- Optional fields are passed through as-is; only present fields are validated in update path

## Filter Pattern
`filterTasks(filter: TaskFilter): Task[]` is the single source of truth for queries.
- All specific getters (`getTasksByStatus`, etc.) should delegate to `filterTasks`
- `TaskFilter` uses optional fields — a missing field means "no constraint on this axis"

## Error Handling
- `ValidationError` (field: string) for bad input
- `throw new Error(\`Task \${id} not found\`)` for missing IDs in update/delete
- API layer catches both and converts to 400/404 responses

## Date Handling
- Store `Date` objects internally, accept ISO strings in input DTOs
- Convert at the service boundary: `new Date(input.dueDate)` in `createTask` / `updateTask`
- All date utilities accept an optional `now` param for testability

## Immutability
- `updateTask` creates a new object with spread: `{ ...existing, ...validated, updatedAt: new Date() }`
- Never mutate tasks in place

## Naming
- Methods on TaskService: camelCase verbs (`createTask`, `filterTasks`, `getDueSoonTasks`)
- Pure utils in `utils/`: standalone exported functions, no class wrappers
- Route handlers: named exports matching HTTP verbs (`createTask`, `listTasks`, `getTask`)
