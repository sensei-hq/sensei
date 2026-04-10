import type { Context, MiddlewareHandler } from "hono";
import { type ZodSchema, ZodError } from "zod";
import { badRequest } from "./errors.js";

/**
 * Middleware factory that parses + validates the JSON request body against
 * a Zod schema. Sets `c.var.body` to the typed, validated value.
 * Throws AppError(400) with Zod field errors on failure.
 *
 * Usage:
 *   app.post('/tasks', validate(CreateTaskSchema), (c) => {
 *     const body = c.var.body as CreateTask;
 *     ...
 *   });
 */
export function validate<T>(schema: ZodSchema<T>): MiddlewareHandler {
  return async (c: Context, next) => {
    let raw: unknown;
    try {
      raw = await c.req.json();
    } catch {
      badRequest("Request body must be valid JSON");
    }

    const result = schema.safeParse(raw);
    if (!result.success) {
      const issues = (result.error as ZodError).issues.map((i) => ({
        field: i.path.join("."),
        message: i.message,
      }));
      badRequest("Validation failed", issues);
    }

    c.set("body", result.data);
    await next();
  };
}

/** Parse and validate query params against a Zod schema. */
export function parseQuery<T>(c: Context, schema: ZodSchema<T>): T {
  const raw = Object.fromEntries(new URL(c.req.url).searchParams);
  const result = schema.safeParse(raw);
  if (!result.success) {
    const issues = (result.error as ZodError).issues.map((i) => ({
      field: i.path.join("."),
      message: i.message,
    }));
    badRequest("Invalid query parameters", issues);
  }
  return result.data;
}
