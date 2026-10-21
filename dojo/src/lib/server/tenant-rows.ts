// Two steps every tenant-addressed admin write shares (#307 A3): the row a route
// names must be the addressed dōjō's — an admin of two dōjōs must not reach one
// through the other's URL — and a database function's refusal keeps the status
// it meant (db-refusal). Fails closed: a failed lookup is a 500, never a 404.
import { AdminError, type DojoClient } from './admin-data';
import { refusalStatus } from './db-refusal';

/** Call a database function; a refusal becomes an AdminError with its status. */
export async function callFn<T = unknown>(db: DojoClient, fn: string, args: Record<string, unknown>): Promise<T> {
	const { data, error } = await db.rpc(fn, args);
	if (error) throw new AdminError(refusalStatus(error), error.message);
	return data as T;
}

/** Require that `table`'s row `id` belongs to the dōjō `tenantId`, else a 404 naming `what`. */
export async function requireInTenant(
	db: DojoClient,
	table: string,
	tenantId: string,
	id: string,
	what: string
): Promise<void> {
	const { data, error } = await db.from(table).select('id').eq('tenant_id', tenantId).eq('id', id).maybeSingle();
	if (error) throw new AdminError(500, error.message);
	if (!data) throw new AdminError(404, `no such ${what} in this dōjō`);
}
