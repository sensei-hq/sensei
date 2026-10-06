// Diagrams · Structure — what the diagram is showing, and what it is NOT.
//
// ## Three states, never two
//
// Loading, error and empty are distinct here because collapsing any pair of
// them produces a specific lie. An error rendered as an empty graph reads as an
// unindexed project; an empty graph rendered as loading never settles. The
// daemon already refuses to return an empty payload on failure (it 500s), and
// this is the client half of that contract — which is why the fetch goes
// through `tryGet`'s Result rather than the fallback-returning `get`, whose
// whole behaviour is to absorb a failure into a sentinel.
//
// ## Coverage is part of the reading, not a footnote
//
// Calls on this corpus are under half placed, so the diagram necessarily omits
// more edges than it draws. A sparse picture with no count beside it reads as a
// simple codebase rather than an unresolved one, so `unplaced` travels with the
// payload and is shown whenever the graph is.
//
// ## Degree is counted over what is DRAWN
//
// Not over the whole graph. A degree that included unplaced edges would
// describe a graph the reader cannot see, and the selection panel sits directly
// beside the thing it is supposedly describing.

import type {
  StructureCoverage,
  StructureEdge,
  StructureLevel,
  StructureNode,
  StructurePayload,
} from '$lib/types.js';

/** The levels offered, in coarsening order. The daemon owns what each MEANS —
 *  notably that `module` is the module's top segment, not the whole path. */
export const LEVELS: StructureLevel[] = ['file', 'module', 'package'];

/** The edge kinds `structure_edges` carries. `calls` alone by default: all five
 *  at file level is the hairball the spec names as a wrong gate. */
export const KINDS = ['calls', 'references', 'imports', 'implements', 'extends'] as const;

/** Holten's 0.85, the mockup's `stBeta` default. */
export const DEFAULT_TENSION = 0.85;

/** The API surface this controller needs — the seam, so it unit-tests without a
 *  daemon and never reaches for a global. */
export interface StructureApi {
  tryGetProjectStructure: (
    id: string,
    level: StructureLevel,
    kinds: string[],
  ) => Promise<
    | { ok: true; data: StructurePayload }
    | { ok: false; error: { status: number; message: string } }
  >;
}

export type Degree = { inbound: number; outbound: number; crossing: number };

export class StructureState {
  #api: StructureApi;
  /** A GETTER, not a snapshot. SvelteKit reuses this component across a param
   *  change, so capturing the id once would leave a second project's screen
   *  querying the first one's. */
  #projectId: () => string;

  level = $state<StructureLevel>('file');
  kinds = $state<string[]>(['calls']);
  bundleTension = $state(DEFAULT_TENSION);
  selected = $state<string | null>(null);

  payload = $state<StructurePayload | null>(null);
  failure = $state<string | null>(null);
  loading = $state(true);

  constructor(api: StructureApi, projectId: () => string) {
    this.#api = api;
    this.#projectId = projectId;
  }

  get nodes(): StructureNode[] {
    return this.payload?.nodes ?? [];
  }

  get edges(): StructureEdge[] {
    return this.payload?.edges ?? [];
  }

  get coverage(): StructureCoverage | null {
    return this.payload?.coverage ?? null;
  }

  /** Loading and error are checked BEFORE empty, so an empty render is only
   *  ever reached when the request succeeded and genuinely returned nothing. */
  get view(): 'loading' | 'error' | 'empty' | 'graph' {
    if (this.loading) return 'loading';
    if (this.failure) return 'error';
    if (this.nodes.length === 0) return 'empty';
    return 'graph';
  }

  get selectedNode(): StructureNode | null {
    if (!this.selected) return null;
    return this.nodes.find((n) => n.id === this.selected) ?? null;
  }

  get degree(): Degree {
    const out: Degree = { inbound: 0, outbound: 0, crossing: 0 };
    if (!this.selected) return out;
    for (const e of this.edges) {
      if (e.source === this.selected) {
        out.outbound += 1;
        // `span` is computed in the view, so "crossing" means the same thing
        // here as it does everywhere else that reads it.
        if (e.span !== 'in_module') out.crossing += 1;
      }
      if (e.target === this.selected) out.inbound += 1;
    }
    return out;
  }

  /** The level control, as `@rokkit/ui`'s `Toolbar` reads it — see the twin on
   *  `LayersState` for why the items are built in state rather than in the
   *  template. */
  get levelItems() {
    return LEVELS.map((level) => ({
      // `label` and NOT `text`: `ProxyItem.label` reads `item[fields.label]`,
      // and the toolbar button renders `proxy.label` for both its visible text
      // and its `aria-label`. An item carrying only `text` renders a button with
      // no words in it at all — verified in the browser before this was fixed.
      label: level,
      value: level,
      active: this.level === level,
      type: 'toggle' as const,
    }));
  }

  /** The edge-kind control. */
  get kindItems() {
    return KINDS.map((kind) => ({
      label: kind,
      value: kind,
      active: this.kinds.includes(kind),
      type: 'toggle' as const,
    }));
  }

  /** "N nodes · M edges drawn · K unplaced (not shown)". The unplaced clause is
   *  unconditional: zero unplaced is itself worth stating, because it is the
   *  one case where the diagram is the whole truth. */
  get coverageLine(): string {
    const unplaced = this.coverage?.unplaced ?? 0;
    return `${this.nodes.length} nodes · ${this.edges.length} edges drawn · ${unplaced} unplaced (not shown)`;
  }

  setLevel(level: StructureLevel) {
    if (this.level === level) return;
    this.level = level;
    // A selection is an id at the OLD level and names nothing at the new one.
    this.selected = null;
  }

  /** The endpoint 400s on an empty kind list, and rightly — "no kinds" is not a
   *  question anyone is asking. Turning the last one off is a no-op rather than
   *  a request that fails. */
  toggleKind(kind: string) {
    const next = this.kinds.includes(kind)
      ? this.kinds.filter((k) => k !== kind)
      : [...this.kinds, kind];
    if (next.length === 0) return;
    this.kinds = next;
  }

  async load() {
    this.loading = true;
    this.failure = null;
    const res = await this.#api.tryGetProjectStructure(
      this.#projectId(),
      this.level,
      this.kinds,
    );
    if (res.ok) {
      this.payload = res.data;
    } else {
      // The previous payload is DROPPED. A stale graph under an error banner
      // invites reading it as current, which is the same error as rendering an
      // empty one — it shows something the daemon did not say.
      this.payload = null;
      this.failure =
        res.error.status === 0
          ? `Could not reach the daemon — ${res.error.message}`
          : `The daemon returned ${res.error.status} ${res.error.message}`;
    }
    this.loading = false;
  }
}

/** Field map for `@rokkit/graph`'s `StructureDiagram`.
 *
 *  `path` is the fqn-derived containment chain the daemon ships, NOT the file
 *  path — a workspace member's directory says nothing about the package it
 *  declares, so a directory-derived rim disagrees with the call graph for every
 *  one of them. */
export const STRUCTURE_FIELDS = {
  id: 'id',
  label: 'label',
  group: 'package',
  weight: 'symbols',
  path: 'path',
  source: 'source',
  target: 'target',
} as const;
