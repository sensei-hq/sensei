import { Hono } from "hono";
import { projects } from "./routes/projects.js";
import { tasks } from "./routes/tasks.js";
import { comments } from "./routes/comments.js";
import { errorHandler } from "./utils/errors.js";

const app = new Hono();

app.route("/projects", projects);
app.route("/tasks", tasks);
app.route("/comments", comments);

app.onError(errorHandler);

app.get("/health", (c) => c.json({ ok: true }));

export default app;

// Start server when run directly (not in tests)
if (import.meta.main) {
  const port = Number(process.env.PORT ?? 3000);
  console.log(`project-board listening on http://localhost:${port}`);
  Bun.serve({ fetch: app.fetch, port });
}
