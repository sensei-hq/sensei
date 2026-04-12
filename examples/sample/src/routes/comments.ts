import { Hono } from "hono";
import { getDb } from "../db.js";
import { notFound } from "../utils/errors.js";

export const comments = new Hono();

comments.delete("/:id", (c) => {
  const db = getDb();
  const id = c.req.param("id");
  if (!db.query("SELECT id FROM comments WHERE id = ?").get(id)) notFound("Comment", id);
  db.run("DELETE FROM comments WHERE id = ?", [id]);
  return c.body(null, 204);
});
