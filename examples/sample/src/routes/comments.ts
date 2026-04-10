import { Hono } from "hono";
import { AppError } from "../utils/errors.js";

export const comments = new Hono();

// TODO: implement DELETE /comments/:id  (see tasks/feature1.md)
// The comments table is defined in src/db.ts.
// Follow the same pattern as tasks.ts: notFound() for missing records, 204 on success.

comments.delete("/:id", (_c) => {
  throw new AppError(501, "Comment delete not yet implemented — see tasks/feature1.md");
});
