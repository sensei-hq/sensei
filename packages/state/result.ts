/** `message` is what a person reads; `body` is the daemon's parsed error body, when a
 *  caller needs more than that (the code it maps, which workspace already owns a board). */
export type ApiError =
  | { status: number; message: string; body?: Record<string, unknown> }
  | { status: 0; message: string };

/** A read or write's answer: the data, or why there is none (never a stand-in value). */
export type ApiResult<T> = { ok: true; data: T } | { ok: false; error: ApiError };

/** Why a read failed, for the screen to say: the status text, else the status. */
export const failureReason = (error: ApiError): string => error.message || `HTTP ${error.status}`;

/** The error for a refused response (#451). The daemon answers `{ error: <code>,
 *  message: <reason> }` on some routes and `{ error: <reason> }` on others: the reason is
 *  what a person reads, the code only when there is no reason, the status text when the
 *  body names neither. A JSON object body is kept for callers that map the code. */
export async function failureFrom(res: Response): Promise<ApiError> {
  let body: Record<string, unknown> | null = null;
  try {
    const parsed: unknown = await res.json();
    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) body = parsed as Record<string, unknown>;
  } catch {
    // A body that is not JSON names no reason: the status text stands.
  }
  if (!body) return { status: res.status, message: res.statusText };
  const text = (key: string) => (typeof body[key] === 'string' && body[key] ? (body[key] as string) : null);
  return { status: res.status, message: text('message') ?? text('error') ?? res.statusText, body };
}
