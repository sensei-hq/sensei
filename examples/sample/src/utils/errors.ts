import type { Context } from "hono";

/**
 * Typed HTTP error. Throw this anywhere in a route handler;
 * the global error handler in index.ts converts it to a JSON response.
 */
export class AppError extends Error {
  constructor(
    public readonly status: number,
    message: string,
    public readonly details?: unknown,
  ) {
    super(message);
    this.name = "AppError";
  }
}

export function notFound(resource: string, id: string): never {
  throw new AppError(404, `${resource} '${id}' not found`);
}

export function badRequest(message: string, details?: unknown): never {
  throw new AppError(400, message, details);
}

/** Global error handler — register with app.onError() in index.ts */
export function errorHandler(err: Error, c: Context) {
  if (err instanceof AppError) {
    return c.json(
      { error: err.message, ...(err.details ? { details: err.details } : {}) },
      err.status as 400 | 404 | 409 | 422 | 500,
    );
  }
  console.error(err);
  return c.json({ error: "Internal server error" }, 500);
}
