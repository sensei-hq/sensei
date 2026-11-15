// Ids a route takes from its path or body. A workspace, tenant or board id is a
// uuid; anything else is the caller's input to fix (400), never a 500 from the
// database's 22P02.

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/** True when `x` is a uuid. */
export function isUuid(x: unknown): x is string {
	return typeof x === 'string' && UUID_RE.test(x);
}
