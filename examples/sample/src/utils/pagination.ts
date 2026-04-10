/**
 * Cursor-based pagination helpers.
 *
 * Cursor encodes the `created_at` timestamp of the last seen item as base64.
 * Queries use `WHERE created_at < :cursor ORDER BY created_at DESC LIMIT :limit`.
 *
 * This gives stable pages even when new items are inserted between requests.
 */

export function decodeCursor(cursor?: string): string | null {
  if (!cursor) return null;
  try {
    return Buffer.from(cursor, "base64url").toString("utf-8");
  } catch {
    return null;
  }
}

export function encodeCursor(createdAt: string): string {
  return Buffer.from(createdAt).toString("base64url");
}

export interface Page<T> {
  data: T[];
  nextCursor: string | null;
}

/**
 * Slice an already-queried result set into a page.
 * Prefer SQL-level pagination (pass LIMIT+cursor to the query) for large tables;
 * this helper is for simple cases where all rows are already in memory.
 */
export function toPage<T extends { created_at: string }>(
  items: T[],
  limit: number,
): Page<T> {
  const data = items.slice(0, limit);
  const nextCursor =
    items.length > limit ? encodeCursor(items[limit - 1].created_at) : null;
  return { data, nextCursor };
}
