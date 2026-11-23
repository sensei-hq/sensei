/** One change a stream reports: what happened to which entity, with its row (or rows).
 *  The scan stream's shape (app `scan-state`); the relay stream follows it (#418). */
export interface StateEvent<T extends { id: string }> {
  action: 'add' | 'update' | 'remove' | 'set';
  entity: string;
  data: T | T[];
}

/** A coded error on a terminal frame. */
export interface StreamFrameError {
  code: string;
  message: string;
}

/** The uniform SSE frame every daemon stream speaks: envelope (`v`, cursor
 *  `id`, emit `ts`) over the event (`entity`, `action`, stable `key`,
 *  payload), with terminal markers only when set. */
export interface StreamFrame {
  v: number;
  id: number;
  ts: string;
  entity: string;
  action: 'add' | 'update' | 'remove' | 'set';
  key: string;
  data: Record<string, unknown>;
  done?: boolean;
  error?: StreamFrameError;
}
