/** `body` is the daemon's parsed error body, when a caller needs more than the
 *  code in `message` (e.g. which workspace already owns a board). */
export type ApiError =
  | { status: number; message: string; body?: Record<string, unknown> }
  | { status: 0; message: string };

/** A read or write's answer: the data, or why there is none (never a stand-in value). */
export type ApiResult<T> = { ok: true; data: T } | { ok: false; error: ApiError };

/** Why a read failed, for the screen to say: the status text, else the status. */
export const failureReason = (error: ApiError): string => error.message || `HTTP ${error.status}`;
