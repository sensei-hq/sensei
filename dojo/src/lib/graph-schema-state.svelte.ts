// One branch's schema for the dōjō's ER lens (#268 slice 5): the read, the
// open tab, the selection, and the canvas settings — the app's SchemaState
// surface over the /v1/you/graph/schema payload. The payload passes through
// to the rokkit views as the design model, never re-keyed.
import { pageFor, type DesignModel } from '@rokkit/graph/schema';
import type { Density, EdgeStyle } from '@rokkit/graph';

export interface SchemaReadApi {
	tryGetSchema: () => Promise<
		{ ok: true; data: Record<string, unknown> } | { ok: false; error: { status: number; message: string } }
	>;
}

export type DesignTabId = 'overview' | 'diagram' | 'entities';

export class GraphSchemaState {
	#api: SchemaReadApi;
	#picked = $state<string | null>(null);

	payload = $state<Record<string, unknown> | null>(null);
	failure = $state<string | null>(null);
	loading = $state(true);
	tab = $state<DesignTabId>('overview');
	density = $state<Density>('keys');
	edgeStyle = $state<EdgeStyle>('curved');
	#focus = $state<string | null>(null);

	constructor(api: SchemaReadApi) {
		this.#api = api;
	}

	get view(): 'loading' | 'error' | 'none' | 'design' {
		if (this.loading) return 'loading';
		if (this.failure) return 'error';
		const p = this.payload as { tables?: unknown[]; entities?: unknown[]; enums?: unknown[] } | null;
		if (!p || ((p.tables ?? []).length === 0 && (p.entities ?? []).length === 0 && (p.enums ?? []).length === 0)) {
			return 'none';
		}
		return 'design';
	}

	get model(): DesignModel | null {
		return this.payload as DesignModel | null;
	}

	get selected(): string | null {
		return this.#picked;
	}

	get page(): 'table' | 'object' | 'enum' | null {
		const m = this.model;
		return m && this.#picked ? (pageFor(m, this.#picked) ?? null) : null;
	}

	select(key: string | null) {
		this.#picked = key;
		this.#focus = null;
	}

	get focus(): string | null {
		return this.#focus;
	}

	focusOn(key: string | null) {
		this.#focus = key;
	}

	async load() {
		this.loading = true;
		this.failure = null;
		const res = await this.#api.tryGetSchema();
		if (res.ok) {
			this.payload = res.data;
		} else {
			this.payload = null;
			this.failure =
				res.error.status === 0
					? `Could not reach the dōjō — ${res.error.message}`
					: `The dōjō returned ${res.error.status} ${res.error.message}`;
		}
		if (this.#picked !== null && this.page === null) this.#picked = null;
		this.loading = false;
	}
}
