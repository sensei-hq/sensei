// Unit tests for the dōjō's schema state (`graph-schema-state.svelte.ts`,
// #268 slice 5): the read, the open tab, the selection, and the canvas
// settings over the /v1/you/graph/schema payload. Mirrors the app's
// SchemaState surface; the API and the failure voice are the dōjō's.
import { describe, it, expect, vi } from 'vitest';
import { GraphSchemaState } from '$lib/graph-schema-state.svelte';

const PAYLOAD = {
	version: 3,
	tables: [{ schema: 'shop', name: 'carts', kind: 'table', columns: [] }],
	refs: [],
	entities: [],
	deps: [],
	enums: []
};

function api(payload: Record<string, unknown> = PAYLOAD, failure: { status: number; message: string } | null = null) {
	return { tryGetSchema: vi.fn(async () => (failure ? { ok: false as const, error: failure } : { ok: true as const, data: payload })) };
}

describe('GraphSchemaState', () => {
	it('starts in loading, before any request has settled', () => {
		const s = new GraphSchemaState(api());
		expect(s.view).toBe('loading');
	});

	it('a failed request is an error with the status, never the none state', async () => {
		const s = new GraphSchemaState(api(PAYLOAD, { status: 403, message: 'no' }));
		await s.load();
		expect(s.view).toBe('error');
		expect(s.failure).toContain('403');
	});

	it('nothing described is the none state; enums alone are something', async () => {
		const empty = new GraphSchemaState(api({ ...PAYLOAD, tables: [] }));
		await empty.load();
		expect(empty.view).toBe('none');
		const enums = new GraphSchemaState(
			api({ ...PAYLOAD, tables: [], enums: [{ schema: 's', name: 'e', values: [] }] })
		);
		await enums.load();
		expect(enums.view).toBe('design');
	});

	it('opens on the Overview and passes the payload through as the model', async () => {
		const s = new GraphSchemaState(api());
		await s.load();
		expect(s.tab).toBe('overview');
		expect(s.model).toEqual(PAYLOAD);
	});

	it('a selection opens its page and clears back to the design', async () => {
		const s = new GraphSchemaState(api());
		await s.load();
		s.select('shop.carts');
		expect(s.page).toBe('table');
		s.select(null);
		expect(s.page).toBeNull();
	});

	it('the canvas centres on what it is given, and opening a page clears it', async () => {
		const s = new GraphSchemaState(api());
		await s.load();
		s.focusOn('shop.carts');
		expect(s.focus).toBe('shop.carts');
		s.select('shop.carts');
		expect(s.focus).toBeNull();
	});
});
