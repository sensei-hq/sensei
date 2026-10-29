// Diagrams · Schema — the workspace's database: tables and their keys, or every
// object and what it depends on (#221).
//
// ## The payload is dbd's, passed through
//
// The daemon serves the ER half of dbd's `SchemaModel` v2 and rokkit's
// `toGraphInput` reads it. This controller FILTERS that model and hands the
// result to `toGraphInput` — it never re-keys a node or renames a field, because
// a field rokkit cannot resolve looks correct while silently never firing.
//
// ## A filter counts what it cut
//
// Choosing a schema hides the others. A key or dependency with one end in a
// hidden schema is a real relationship, so it is not drawn AND it is counted
// beside the picture. Without the count, a schema whose keys all lead elsewhere
// reads as self-contained.
//
// An unresolved dependency (a built-in, an object the store does not describe)
// has no end in ANY schema, so no filter hides it: it stays while its source is
// shown, drawn dimmed by rokkit as `unplaced`.

import { toGraphInput } from '@rokkit/graph/schema';
import type { SchemaDep, SchemaEntity, SchemaPayload, SchemaRef, SchemaTable } from '$lib/types.js';

export type SchemaScope = 'er' | 'dependencies';

export const SCOPES: { value: SchemaScope; label: string }[] = [
  { value: 'er', label: 'Tables & keys' },
  { value: 'dependencies', label: 'Dependencies' },
];

export interface SchemaApi {
  tryGetWorkspaceSchema: (
    id: string,
  ) => Promise<
    { ok: true; data: SchemaPayload } | { ok: false; error: { status: number; message: string } }
  >;
}

export interface SchemaOption {
  /** `null` is All. */
  value: string | null;
  label: string;
  tables: number;
}

/** The id `toGraphInput` gives a node, so a selection and a node agree. */
export function objectId(schema: string, name: string): string {
  return schema ? `${schema}.${name}` : name;
}

function plural(n: number, one: string, many: string): string {
  return `${n.toLocaleString()} ${n === 1 ? one : many}`;
}

export class SchemaState {
  #api: SchemaApi;
  #workspaceId: () => string;

  scope = $state<SchemaScope>('er');
  /** The chosen schema; `null` is All. `load` drops a choice the new payload
   *  does not name. */
  #chosen = $state<string | null>(null);
  #picked = $state<string | null>(null);

  payload = $state<SchemaPayload | null>(null);
  failure = $state<string | null>(null);
  loading = $state(true);

  constructor(api: SchemaApi, workspaceId: () => string) {
    this.#api = api;
    this.#workspaceId = workspaceId;
  }

  setApi(api: SchemaApi) {
    this.#api = api;
  }

  get scopes() {
    return SCOPES;
  }

  get view(): 'loading' | 'error' | 'none' | 'graph' {
    if (this.loading) return 'loading';
    if (this.failure) return 'error';
    const p = this.payload;
    if (!p || (p.tables.length === 0 && p.entities.length === 0)) return 'none';
    return 'graph';
  }

  /** Every schema the payload names, tables first counted. */
  get schemaOptions(): SchemaOption[] {
    const p = this.payload;
    if (!p) return [];
    const counts = new Map<string, number>();
    for (const t of p.tables) counts.set(t.schema, (counts.get(t.schema) ?? 0) + 1);
    for (const e of p.entities) if (!counts.has(e.schema)) counts.set(e.schema, 0);
    const named = [...counts.entries()]
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([name, tables]) => ({ value: name, label: name, tables }));
    return [{ value: null, label: 'All', tables: p.tables.length }, ...named];
  }

  get schema(): string | null {
    return this.#chosen;
  }

  #shows(schema: string): boolean {
    const s = this.schema;
    return s === null || s === schema;
  }

  get #tables(): SchemaTable[] {
    return (this.payload?.tables ?? []).filter((t) => this.#shows(t.schema));
  }

  get #entities(): SchemaEntity[] {
    return (this.payload?.entities ?? []).filter((e) => this.#shows(e.schema));
  }

  /** Keys with both ends shown, and the count with exactly one. */
  get #refs(): { kept: SchemaRef[]; cut: number } {
    let cut = 0;
    const kept: SchemaRef[] = [];
    for (const r of this.payload?.refs ?? []) {
      const ends = [this.#shows(r.from.s), this.#shows(r.to.s)];
      if (ends[0] && ends[1]) kept.push(r);
      else if (ends[0] || ends[1]) cut++;
    }
    return { kept, cut };
  }

  /** Deps with both ends shown — or an unresolved one whose source is — and the
   *  count of placed deps with exactly one end shown. */
  get #deps(): { kept: SchemaDep[]; cut: number } {
    let cut = 0;
    const kept: SchemaDep[] = [];
    for (const d of this.payload?.deps ?? []) {
      const from = this.#shows(d.from.s);
      if (d.unresolved) {
        if (from) kept.push(d);
        continue;
      }
      const to = this.#shows(d.to.s);
      if (from && to) kept.push(d);
      else if (from || to) cut++;
    }
    return { kept, cut };
  }

  /** Links the schema filter cut, in the scope being drawn. */
  get cut(): number {
    return this.scope === 'er' ? this.#refs.cut : this.#deps.cut;
  }

  /** `toGraphInput` over the filtered model — nodes, edges and dbd's field map. */
  get graphInput() {
    const model = {
      tables: this.#tables,
      refs: this.#refs.kept,
      entities: this.#entities,
      deps: this.#deps.kept,
    };
    return toGraphInput(model, this.scope);
  }

  /** One fact per line. */
  get coverage(): string[] {
    // What the picture in view draws: keys are not drawn under Dependencies,
    // and views and routines are not drawn under Keys.
    const lines = [
      this.scope === 'er'
        ? `${plural(this.#tables.length, 'table', 'tables')} · ${plural(this.#refs.kept.length, 'key', 'keys')}`
        : `${plural(this.#tables.length + this.#entities.length, 'object', 'objects')} · ${plural(this.#deps.kept.length, 'dependency', 'dependencies')}`,
    ];
    const cut = this.cut;
    if (cut > 0) {
      lines.push(
        this.scope === 'er'
          ? `${plural(cut, 'key leads', 'keys lead')} out of the shown schema, not drawn`
          : `${plural(cut, 'dependency crosses', 'dependencies cross')} out of the shown schema, not drawn`,
      );
    }
    if (this.scope === 'dependencies') {
      const dim = this.#deps.kept.filter((d) => d.unresolved).length;
      if (dim > 0) {
        lines.push(
          `${plural(dim, 'dependency', 'dependencies')} on a built-in or undescribed object, drawn dimmed`,
        );
      }
    }
    return lines;
  }

  /** The selected table's id, or null. A selection the filter hides is gone. */
  get selected(): string | null {
    const id = this.#picked;
    if (id === null) return null;
    return this.#tables.some((t) => objectId(t.schema, t.name) === id) ? id : null;
  }

  get selectedTable(): SchemaTable | null {
    const id = this.selected;
    return id === null ? null : (this.#tables.find((t) => objectId(t.schema, t.name) === id) ?? null);
  }

  /** What a change to the selected table reaches, over the whole model — a key
   *  or a reader in another schema still counts: the tables it references, those
   *  referencing it (each with the key column), and the objects that depend on it. */
  get reach(): { references: string[]; referencedBy: string[]; dependents: string[] } | null {
    const t = this.selectedTable;
    if (!t) return null;
    const is = (s: string, n: string) => s === t.schema && n === t.name;
    const refs = this.payload?.refs ?? [];
    return {
      references: refs.filter((r) => is(r.from.s, r.from.t)).map((r) => `${r.to.s}.${r.to.t} (${r.from.c})`),
      referencedBy: refs.filter((r) => is(r.to.s, r.to.t)).map((r) => `${r.from.s}.${r.from.t} (${r.from.c})`),
      dependents: (this.payload?.deps ?? [])
        .filter((d) => !d.unresolved && is(d.to.s, d.to.n))
        .map((d) => `${d.from.s}.${d.from.n} ${d.kind}`),
    };
  }

  select(id: string | null) {
    this.#picked = id;
  }

  setScope(scope: SchemaScope) {
    this.scope = scope;
  }

  setSchema(schema: string | null) {
    this.#chosen = schema;
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
    // A choice the new payload does not name falls back to All: an empty
    // picture for a schema that is gone answers nothing.
    if (this.#chosen !== null && !this.schemaOptions.some((o) => o.value === this.#chosen)) {
      this.#chosen = null;
    }
    this.loading = false;
  }
}
