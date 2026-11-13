/** One change a stream reports: what happened to which entity, with its row (or rows).
 *  The scan stream's shape (app `scan-state`); the relay stream follows it (#418). */
export interface StateEvent<T extends { id: string }> {
  action: 'add' | 'update' | 'remove' | 'set';
  entity: string;
  data: T | T[];
}
