// Diagrams · World — all indexed code, nested by what contains it.
//
// ## The picture is WIDER than the project it is opened from
//
// Project is the outermost ring, so scoping to one would leave a single circle.
// That makes one thing non-optional: the screen has to say which circle the
// reader is standing in, or a view of every project silently reads as a view of
// theirs.
//
// ## Drill-down is a PATH
//
// `@rokkit/graph`'s `world` layout takes `focusPath`, and every node carries its
// own full path including itself. A path cannot dangle the way a parent id can,
// and it survives a re-fetch without a lookup — so the focus lives here as a
// path and nowhere as an id.
//
// A GROUPING CHANGE DROPS THE FOCUS. The orderings are permutations, so
// `['alpha', 'core']` names a circle under `project` and nothing under
// `repository`. Keeping it would focus the picture on something that no longer
// exists.
//
// ## `nothing` is the default shade, and a choice
//
// The picture's first job is size. Colouring by a measure before a reader has
// asked turns every glance into an interpretation of a number they did not pick.

import type {
  WorldGroupBy,
  WorldPayload,
  WorldShadeBy,
  WorldTotals,
  WorldUnit,
} from '$lib/types.js';

/** The groupings offered, in the order the control shows them. */
export const GROUPINGS: { value: WorldGroupBy; label: string }[] = [
  { value: 'project', label: 'Project' },
  { value: 'repository', label: 'Repository' },
  // NOT "code · tests · docs". Documentation is not a declaration, so a docs
  // ring would always be empty — see `$lib/types.ts` and #246.
  { value: 'kind', label: 'Code · tests' },
];

/** What a circle can be coloured by. */
export const SHADES: { value: WorldShadeBy; label: string }[] = [
  { value: 'nothing', label: 'Nothing' },
  { value: 'unresolvedShare', label: 'Unresolved share' },
  { value: 'testShare', label: 'Test share' },
  { value: 'documentedShare', label: 'Documented share' },
];

/** What the root of the picture is called. It is not a circle in the payload —
 *  it is everything — so it needs a name of its own rather than an empty one. */
export const ROOT_LABEL = 'All indexed code';

export interface WorldApi {
  tryGetProjectWorld: (
    id: string,
    groupBy: WorldGroupBy,
  ) => Promise<
    { ok: true; data: WorldPayload } | { ok: false; error: { status: number; message: string } }
  >;
}

/** The focused circle, which at the root is the whole corpus. */
export interface InView {
  label: string;
  path: string[];
  weight: number;
  measures: WorldUnit['measures'];
}

export class WorldState {
  #api: WorldApi;
  #projectId: () => string;

  groupBy = $state<WorldGroupBy>('project');
  shadeBy = $state<WorldShadeBy>('nothing');
  /** Empty is the root — the whole corpus, not "no selection". */
  focusPath = $state<string[]>([]);

  payload = $state<WorldPayload | null>(null);
  failure = $state<string | null>(null);
  loading = $state(true);

  constructor(api: WorldApi, projectId: () => string) {
    this.#api = api;
    this.#projectId = projectId;
  }

  setApi(api: WorldApi) {
    this.#api = api;
  }

  /** The control copy, read through the controller so a template never imports
   *  a second list and the two cannot drift. */
  get groupings() {
    return GROUPINGS;
  }

  get shades() {
    return SHADES;
  }

  get units(): WorldUnit[] {
    return this.payload?.units ?? [];
  }

  get totals(): WorldTotals | null {
    return this.payload?.totals ?? null;
  }

  get view(): 'loading' | 'error' | 'empty' | 'graph' {
    if (this.loading) return 'loading';
    if (this.failure) return 'error';
    if (this.units.length === 0) return 'empty';
    return 'graph';
  }

  /** The measure key to colour by, or `undefined` for no shading at all.
   *  `undefined` rather than `'nothing'` because that is what the component
   *  takes — a key it cannot find would shade every circle alike. */
  get shadeKey(): string | undefined {
    return this.shadeBy === 'nothing' ? undefined : this.shadeBy;
  }

  /** The focused circle, or the whole corpus at the root.
   *
   *  The root is SYNTHESISED rather than looked up: it is not a unit in the
   *  payload, because nothing contains everything. Its weight is the corpus
   *  total and its measures are the corpus shares, so the panel reads the same
   *  way at every depth. */
  get inView(): InView {
    const found = this.units.find((u) => samePath(u.path, this.focusPath));
    if (found) {
      return {
        label: found.label,
        path: found.path,
        weight: found.weight,
        measures: found.measures,
      };
    }
    const totals = this.totals;
    const declarations = totals?.declarations ?? 0;
    const share = (n: number) => (declarations === 0 ? 0 : n / declarations);
    return {
      label: ROOT_LABEL,
      path: [],
      weight: declarations,
      measures: {
        documentedShare: share(totals?.documented ?? 0),
        testShare: share(totals?.tests ?? 0),
        // The corpus-wide unresolved share is not derivable from the totals —
        // they count declarations, not edges — so it is absent rather than
        // computed from the wrong denominator.
        unresolvedShare: null,
      },
    };
  }

  /** What sits DIRECTLY inside the focused circle, biggest first.
   *
   *  Children only. A list that included grandchildren would double-count what
   *  it is describing, because every container's weight already holds them. */
  get largestInside(): WorldUnit[] {
    const depth = this.focusPath.length;
    return this.units
      .filter((u) => u.path.length === depth + 1 && samePath(u.path.slice(0, depth), this.focusPath))
      .slice()
      .sort((a, b) => b.weight - a.weight || a.label.localeCompare(b.label));
  }

  /** "Standing in <project>, of N projects indexed." */
  get viewingNote(): string {
    const viewing = this.payload?.viewing ?? '—';
    const projects = this.totals?.projects ?? 0;
    return `Everything indexed, not only this project — you opened it from ${viewing}, one of ${projects} projects.`;
  }

  get totalsLine(): string {
    const t = this.totals;
    if (!t) return '';
    return `${t.declarations.toLocaleString()} declarations · ${t.repositories} repositories · ${t.projects} projects`;
  }

  /** One measure of the focused circle, in words.
   *
   *  ABSENT IS SAID, not printed as zero. `unresolvedShare` is null below the
   *  repository ring because edges belong to a folder and declarations to a
   *  code-or-tests cell; a "0%" there would read as perfectly resolved. */
  measureNote(measure: Exclude<WorldShadeBy, 'nothing'>): string {
    const value = this.inView.measures[measure];
    if (value === null || value === undefined) {
      return 'not a fact at this level — edges belong to a repository, not to its code or tests half';
    }
    return `${Math.round(value * 100)}%`;
  }

  /** The units as the LAYOUT needs them: weight on leaves only (#219).
   *
   *  rokkit's world layout treats a declared container's `weight` as its OWN
   *  measure and sums its children on top. A container here holds no
   *  declarations of its own — its payload weight is the TOTAL of what it
   *  contains, which is what the IN VIEW panel reads — so passing it through
   *  counted every declaration once per ring: 27,295 drew as "81.9K". */
  get layoutUnits(): WorldUnit[] {
    const keys = this.units.map((u) => u.path.join('\u0000'));
    const containers = new Set(
      keys.filter((k) => keys.some((other) => other.startsWith(k + '\u0000'))),
    );
    return this.units.map((u) =>
      containers.has(u.path.join('\u0000')) ? { ...u, weight: 0 } : u,
    );
  }

  /** The WHOLE `GraphState` config, every time (#219).
   *
   *  `GraphState.update` replaces the config: anything it is not handed reverts
   *  to the default layout. The page used to pass nodes, shade and focus alone,
   *  which threw `layout: 'world'` away on the first update and drew the circle
   *  pack as a stack of cards. One getter that names everything means no update
   *  can forget a part. */
  get graphConfig() {
    return {
      fields: WORLD_FIELDS,
      layout: 'world' as const,
      sizeBy: 'weight',
      sizeScale: 'log' as const,
      nodes: this.layoutUnits,
      shadeBy: this.shadeKey,
      focusPath: this.focusPath,
      // A caller-supplied GraphState takes its callbacks from its OWN config —
      // `<Graph ondrill>` is wired only into the state Graph creates for itself.
      // Without these a canvas drill moved the picture and left the panel at the
      // root (#219, seen live).
      ondrill: (path: string[]) => this.drillTo(path),
      ondrillup: (path: string[]) => this.drillTo(path),
      onfocuspath: (path: string[]) => this.drillTo(path),
    };
  }

  drillTo(path: string[]) {
    this.focusPath = [...path];
  }

  /** Out one ring. At the root this is a no-op rather than a path that names
   *  nothing. */
  drillUp() {
    this.focusPath = this.focusPath.slice(0, -1);
  }

  setGroupBy(groupBy: WorldGroupBy) {
    if (this.groupBy === groupBy) return;
    this.groupBy = groupBy;
    // The orderings are permutations, so a path from the old one names nothing
    // in the new. Keeping it would focus on a circle that does not exist.
    this.focusPath = [];
  }

  setShadeBy(shadeBy: WorldShadeBy) {
    this.shadeBy = shadeBy;
  }

  async load() {
    this.loading = true;
    this.failure = null;
    const res = await this.#api.tryGetProjectWorld(this.#projectId(), this.groupBy);
    if (res.ok) {
      this.payload = res.data;
    } else {
      this.payload = null;
      this.failure =
        res.error.status === 0
          ? `Could not reach the daemon — ${res.error.message}`
          : `The daemon returned ${res.error.status} ${res.error.message}`;
    }
    this.loading = false;
  }
}

function samePath(a: string[], b: string[]): boolean {
  return a.length === b.length && a.every((segment, i) => segment === b[i]);
}

/** Field map for `@rokkit/graph`'s `Graph` under the `world` layout.
 *
 *  `path` and `measures` are the two that matter: the first is what nests a
 *  circle, the second is the bag the shade control names a key in. Both are
 *  passed through as the endpoint spells them, so a renamed field is a compile
 *  error in `$lib/types.ts` rather than a silently flat picture. */
export const WORLD_FIELDS = {
  id: 'id',
  label: 'label',
  path: 'path',
  weight: 'weight',
  measures: 'measures',
} as const;
