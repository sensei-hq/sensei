/**
 * Kavach-style data endpoint for the sensei schema.
 *
 * GET /data?entity=<table>&:select=<cols>&:order=<col:dir>&:limit=<n>&<field>=<op>.<value>
 *
 * Mirrors the URL convention from kavach's handleDataRoute:
 *   - `:select`, `:order`, `:limit`, `:offset`, `:count` are query options (reserved params)
 *   - All other params are treated as filters in PostgREST format: `field=op.value`
 *
 * Uses @kavach/adapter-supabase getActions with the service-role client
 * so the sensei schema is accessible without RLS restrictions.
 */
import type { RequestHandler } from './$types';
import { json } from '@sveltejs/kit';
import { getDb } from '$lib/server/db';
import { getActions } from '@kavach/adapter-supabase';

const ALLOWED_ENTITIES = new Set([
  'repos',
  'task_sessions',
  'sessions',
  'libraries',
  'symbols',
  'scan_state',
  'snapshots',
  'memory_items',
  'api_requests',
  'benchmark_runs',
  'referenced_libraries',
]);

export const GET: RequestHandler = async ({ url }) => {
  const entity = url.searchParams.get('entity');

  if (!entity) {
    return json({ error: { message: 'Missing required query param: entity' } }, { status: 400 });
  }
  if (!ALLOWED_ENTITIES.has(entity)) {
    return json({ error: { message: `Entity not allowed: ${entity}` } }, { status: 400 });
  }

  // Extract reserved query options (prefixed with ':')
  const select  = url.searchParams.get(':select') ?? undefined;
  const order   = url.searchParams.get(':order')  ?? undefined;
  const limitRaw  = url.searchParams.get(':limit');
  const offsetRaw = url.searchParams.get(':offset');
  const count   = url.searchParams.get(':count')  ?? undefined;

  // Remaining params become filters in PostgREST format: { field: 'op.value' }
  const filter: Record<string, string> = {};
  for (const [key, value] of url.searchParams.entries()) {
    if (!key.startsWith(':') && key !== 'entity') {
      filter[key] = value;
    }
  }

  try {
    const db = getDb();
    const actions = getActions(db, 'sensei');

    const result = await actions.get(entity, {
      columns: select,
      order,
      limit:  limitRaw  !== null ? Number(limitRaw)  : undefined,
      offset: offsetRaw !== null ? Number(offsetRaw) : undefined,
      count,
      filter: Object.keys(filter).length > 0 ? filter : undefined,
    });

    if (result.error) {
      return json({ error: result.error }, { status: result.status ?? 500 });
    }

    return json(
      result.count !== undefined
        ? { data: result.data, count: result.count }
        : { data: result.data }
    );
  } catch (err) {
    const message = err instanceof Error ? err.message : 'Internal server error';
    return json({ error: { message } }, { status: 500 });
  }
};
