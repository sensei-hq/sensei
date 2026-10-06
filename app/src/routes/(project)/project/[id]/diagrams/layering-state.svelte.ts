// Diagrams · Layers and Cycles — what the layering says, and what it does not.
//
// ## One controller, two screens
//
// The layering, the cycles and the coverage all come out of a single `analyse`
// over one read of `structure_edges`, and that read is the entire cost of the
// request (1.5 s to 74 s per project). Two controllers would pay it twice for
// two answers that must agree about which arrow is the weakest link.
//
// ## `skip` IS NOT A VIOLATION
//
// The one reading this screen can get wrong in a way that matters. A call that
// skips a layer is legal under relaxed layering, and `@rokkit/graph` keeps the
// two apart in its own vocabulary for exactly that reason (`layout/edges.js`
// filters `conformance === 'up'`, not `!== 'down'`). Counting a skip as a break
// reports a problem the architecture does not have, so [`LayersState.breaks`]
// is the only place the question is asked and every surface reads it.
//
// ## Zero breaks is a FINDING, not an absence
//
// An empty violations list under an empty caption reads as "this architecture
// is clean" whether the layering found nothing or the request returned nothing.
// So `breaksNote` always has words in it, and the three view states stay
// distinct for the same reason they do on Structure.
//
// ## Derived, not declared
//
// `layerSource: "derived"` means this layering was MEASURED from the call graph.
// The screen therefore answers *do these calls flow downward*, and not *is this
// the architecture you intended* — which needs a layering somebody declared.

import type {
  LayeringCoverage,
  LayeringCycle,
  LayeringDep,
  LayeringEdge,
  LayeringLevel,
  LayeringNode,
  LayeringPayload,
} from '$lib/types.js';

/** The grains offered. `package` is absent because the daemon refuses it: a
 *  package graph on this corpus is single digits of nodes, which has no
 *  layering to show. */
export const LEVELS: LayeringLevel[] = ['module', 'file'];

/** The kinds the layering can be computed over. `calls` alone by default — it
 *  is the one that means "depends on at runtime", and the others turn a
 *  layering into a reachability picture. */
export const KINDS = ['calls', 'references', 'imports', 'implements', 'extends'] as const;

/** The API surface this controller needs — the seam, so it unit-tests without
 *  a daemon and never reaches for a global. */
export interface LayeringApi {
  tryGetProjectLayering: (
    id: string,
    level: LayeringLevel,
    kinds: string[],
  ) => Promise<
    { ok: true; data: LayeringPayload } | { ok: false; error: { status: number; message: string } }
  >;
}

export class LayersState {
  #api: LayeringApi;
  /** A GETTER, not a snapshot. SvelteKit reuses this component across a param
   *  change, so capturing the id once would leave a second project's screen
   *  querying the first one's. */
  #projectId: () => string;

  level = $state<LayeringLevel>('module');
  kinds = $state<string[]>(['calls']);
  /** Every edge, or only the ones that climb — the value `ViolationsControl`
   *  moves and `LayersDiagram` binds. */
  showEdges = $state<'all' | 'violations'>('all');
  selected = $state<string | null>(null);

  payload = $state<LayeringPayload | null>(null);
  failure = $state<string | null>(null);
  loading = $state(true);

  constructor(api: LayeringApi, projectId: () => string) {
    this.#api = api;
    this.#projectId = projectId;
  }

  /** Swap the seam. For tests that need a second request to behave differently
   *  from the first; production builds one controller per screen. */
  setApi(api: LayeringApi) {
    this.#api = api;
  }

  get nodes(): LayeringNode[] {
    return this.payload?.nodes ?? [];
  }

  get edges(): LayeringEdge[] {
    return this.payload?.edges ?? [];
  }

  get coverage(): LayeringCoverage | null {
    return this.payload?.coverage ?? null;
  }

  get depth(): number {
    return this.payload?.depth ?? 0;
  }

  get selfDependencies(): LayeringDep[] {
    return this.payload?.selfDependencies ?? [];
  }

  /** Loading and error are checked BEFORE empty, so an empty render is only
   *  ever reached when the request succeeded and genuinely returned nothing. */
  get view(): 'loading' | 'error' | 'empty' | 'graph' {
    if (this.loading) return 'loading';
    if (this.failure) return 'error';
    if (this.nodes.length === 0) return 'empty';
    return 'graph';
  }

  get selectedNode(): LayeringNode | null {
    if (!this.selected) return null;
    return this.nodes.find((n) => n.id === this.selected) ?? null;
  }

  /** The calls that BREAK the downward flow, and nothing else.
   *
   *  `up` only. A `skip` jumps a layer and is legal under relaxed layering;
   *  `level` is a call inside one layer, which a cycle's members make
   *  constantly. Every `up` edge is a cycle's back edge, which is why the
   *  cut named on the Cycles screen and the arrows highlighted here are the
   *  same set of facts. */
  get breaks(): LayeringEdge[] {
    return this.edges.filter((e) => e.conformance === 'up');
  }

  /** Cycles top-down: the one everything else depends on first, and the bigger
   *  of two at one depth before the smaller. The condensation reads downward,
   *  so an arbitrary order would put the leaf cycle above its dependents. */
  get cycles(): LayeringCycle[] {
    return [...(this.payload?.cycles ?? [])].sort(
      (a, b) => a.layer - b.layer || b.members.length - a.members.length,
    );
  }

  /** "N units · M dependencies drawn · K with an endpoint that owns no file".
   *
   *  The last clause is unconditional. Measured on the largest client project,
   *  95 of 122 dependencies are in that state — a diagram missing four fifths
   *  of its edges with no number beside it reads as a simple codebase. */
  get coverageLine(): string {
    const c = this.coverage;
    const units = c?.units ?? this.nodes.length;
    const drawn = c?.drawn ?? this.edges.length;
    const unknown = c?.unknownUnit ?? 0;
    return `${units} units · ${drawn} dependencies drawn · ${unknown} with an endpoint that owns no file`;
  }

  /** The grain control, as `@rokkit/ui`'s `Toolbar` reads it.
   *
   *  BUILT HERE because the component is a pure template and the toolbar is
   *  data: `text`, `value`, `active` and `type` are the default field names, so
   *  no `fields` map is needed. `toggle` rather than `button` is what gives each
   *  item `aria-pressed` and `data-active`. */
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

  /** The edge-kind control. Multi-select, where the grain is single-select —
   *  which is a property of the HANDLER and not of the item shape, so the two
   *  bars render through one component and dispatch differently. */
  get kindItems() {
    return KINDS.map((kind) => ({
      label: kind,
      value: kind,
      active: this.kinds.includes(kind),
      type: 'toggle' as const,
    }));
  }

  /** What `layerSource` means, in words, because the distinction decides which
   *  question the reader may ask of the picture. */
  get sourceNote(): string {
    return `Layering measured from the call graph, ${this.depth} ${this.depth === 1 ? 'layer' : 'layers'} deep — not a layering anyone declared.`;
  }

  /** Always a sentence, never an empty list under an empty caption. */
  get breaksNote(): string {
    const n = this.breaks.length;
    if (n === 0) {
      return 'No call climbs a layer. Every dependency points down or sits inside one.';
    }
    return `${n} ${n === 1 ? 'call climbs' : 'calls climb'} a layer. Each one is a cycle's back edge — the arrow to cut.`;
  }

  /** The mockup's list row, built once so the two screens cannot word it
   *  differently.
   *
   *  A cycle with no cut is a real state and gets its own sentence: saying
   *  nothing would read as a loop that needs no action. */
  cutLine(cycle: LayeringCycle | undefined): string {
    if (!cycle) return '';
    if (!cycle.cut) {
      return 'No single dependency opens this loop — every edge in it carries the same weight.';
    }
    const { source, target, occurrences } = cycle.cut;
    return `Cut ${source} → ${target} (×${occurrences}, the weakest link) and the loop opens.`;
  }

  setLevel(level: LayeringLevel) {
    if (this.level === level) return;
    this.level = level;
    // A selection is an id at the OLD grain and names nothing at the new one.
    this.selected = null;
  }

  /** The endpoint 400s on an empty kind list, and rightly — "no kinds" is not
   *  a question anyone is asking. Turning the last one off is a no-op rather
   *  than a request that fails. */
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
    const res = await this.#api.tryGetProjectLayering(this.#projectId(), this.level, this.kinds);
    if (res.ok) {
      this.payload = res.data;
    } else {
      // The previous payload is DROPPED. A stale layering under an error banner
      // invites reading it as current, which is worse here than on Structure:
      // the arrows it highlights are the ones somebody is about to go and cut.
      this.payload = null;
      this.failure =
        res.error.status === 0
          ? `Could not reach the daemon — ${res.error.message}`
          : `The daemon returned ${res.error.status} ${res.error.message}`;
    }
    this.loading = false;
  }
}

/** Field map for `@rokkit/graph`'s `LayersDiagram`.
 *
 *  `layer` is NOT listed and that is deliberate: the component ranks by
 *  `node.layer` by name, and derives `conformance` itself with the identical
 *  rule when a host omits it — so the daemon's value and the component's
 *  cannot disagree about which arrow climbs. */
export const LAYERS_FIELDS = {
  id: 'id',
  label: 'label',
  group: 'group',
  weight: 'weight',
  source: 'source',
  target: 'target',
} as const;
