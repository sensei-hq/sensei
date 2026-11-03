// Diagrams · Database · ER — the workspace's database as dbd's viewer shows it
// (user, 2026-11-02): the design's Overview, Diagram and Entities, or one
// entity's page (a table, an object, an enum), with the sidebar on the right.
//
// ## The payload is dbd's, passed through
//
// The daemon serves dbd's `SchemaModel` v3 (tables, refs, entities, deps,
// enums); rokkit's `@rokkit/graph/schema` views read it as a `DesignModel`
// unchanged — never re-keyed or renamed, because a field rokkit cannot resolve
// looks correct while silently never firing. This state owns only what the page
// does: the read, the open tab, the selection, and the diagram's settings.

import { pageFor, type DesignModel } from '@rokkit/graph/schema';
import type { Density, EdgeStyle } from '@rokkit/graph';
import type { SchemaPayload } from '$lib/types.js';

export interface SchemaApi {
  tryGetWorkspaceSchema: (
    id: string,
  ) => Promise<
    { ok: true; data: SchemaPayload } | { ok: false; error: { status: number; message: string } }
  >;
}

/** The design-level tabs (dbd's; Changelog waits for dbd release 1). */
export type DesignTabId = 'overview' | 'diagram' | 'entities';

export class SchemaState {
  #api: SchemaApi;
  #workspaceId: () => string;
  #picked = $state<string | null>(null);

  payload = $state<SchemaPayload | null>(null);
  failure = $state<string | null>(null);
  loading = $state(true);
  tab = $state<DesignTabId>('overview');
  /** The ER diagram's field detail and edge style — rokkit's own settings. */
  density = $state<Density>('keys');
  edgeStyle = $state<EdgeStyle>('curved');

  constructor(api: SchemaApi, workspaceId: () => string) {
    this.#api = api;
    this.#workspaceId = workspaceId;
  }

  setApi(api: SchemaApi) {
    this.#api = api;
  }

  get view(): 'loading' | 'error' | 'none' | 'design' {
    if (this.loading) return 'loading';
    if (this.failure) return 'error';
    const p = this.payload;
    if (!p || (p.tables.length === 0 && p.entities.length === 0 && (p.enums ?? []).length === 0)) return 'none';
    return 'design';
  }

  /** The payload as rokkit's views read it. */
  get model(): DesignModel | null {
    return this.payload as DesignModel | null;
  }

  get selected(): string | null {
    return this.#picked;
  }

  /** Which page the selection opens; null is the design itself. */
  get page(): 'table' | 'object' | 'enum' | null {
    const m = this.model;
    return m && this.#picked ? (pageFor(m, this.#picked) ?? null) : null;
  }

  select(key: string | null) {
    this.#picked = key;
  }

  async load() {
    this.loading = true;
    this.failure = null;
    const res = await this.#api.tryGetWorkspaceSchema(this.#workspaceId());
    if (res.ok) {
      this.payload = res.data;
    } else {
      this.payload = null;
      this.failure =
        res.error.status === 0
          ? `Could not reach the daemon — ${res.error.message}`
          : `The daemon returned ${res.error.status} ${res.error.message}`;
    }
    // A selection the new design does not have clears: a page for nothing
    // answers nothing.
    if (this.#picked !== null && this.page === null) this.#picked = null;
    this.loading = false;
  }
}
