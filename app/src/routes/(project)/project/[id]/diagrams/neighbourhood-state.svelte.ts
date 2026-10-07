// Diagrams · Neighbourhood — one symbol, what calls it and what it calls.
//
// ## There is no default symbol
//
// The screen opens on a picker. Choosing a symbol for the reader — the most
// called, the first alphabetically — would draw a neighbourhood they did not
// ask about, and a confident picture of the wrong thing is worse than a
// question.
//
// ## Depth re-asks the daemon
//
// `@rokkit/graph`'s `Neighborhood` can walk rings itself, but only over the
// edges it was given. The daemon caps each ring and counts what it cut, so the
// depth is a request parameter: asking for three rings up front would fetch
// the largest picture for every reader who only wanted one.
//
// ## What the picture cannot show is said beside it
//
// Three separate facts, never summed: cards a capped ring did not draw, calls
// the focus makes that the graph could not place, and unplaced calls that use
// the focus's NAME. The last is a "may", so it is worded as one.

import type { ApiResult } from '$lib/api.js';
import type {
  FunctionMatch,
  NeighbourCard,
  NeighbourEdge,
  NeighbourhoodPayload,
} from '$lib/types.js';

export const MIN_DEPTH = 1;
export const MAX_DEPTH = 3;

/** Characters before the picker searches. One matches half the corpus. */
const MIN_QUERY = 2;

export interface NeighbourhoodApi {
  tryGetProjectNeighbourhood: (
    id: string,
    focus: string,
    depth: number,
  ) => Promise<ApiResult<NeighbourhoodPayload>>;
  trySearchFunctions: (id: string, q: string) => Promise<ApiResult<FunctionMatch[]>>;
}

/** A card as the component draws it: the payload's, plus where to find it. */
export type DrawnCard = NeighbourCard & { note?: string };

function failureText(error: { status: number; message: string }, notFound: string): string {
  if (error.status === 0) return `Could not reach the daemon — ${error.message}`;
  if (error.status === 404) return notFound;
  return `The daemon returned ${error.status} ${error.message}`;
}

function plural(n: number, one: string, many: string): string {
  return `${n.toLocaleString()} ${n === 1 ? one : many}`;
}

export class NeighbourhoodState {
  #api: NeighbourhoodApi;
  #projectId: () => string;
  /** Bumped per request, so a slow answer for an old focus is dropped. */
  #ticket = 0;

  focus = $state<string | null>(null);
  depth = $state(MIN_DEPTH);

  payload = $state<NeighbourhoodPayload | null>(null);
  failure = $state<string | null>(null);
  loading = $state(false);
  /** Why the last re-centre was refused, shown until the next one succeeds. */
  refusal = $state<string | null>(null);

  query = $state('');
  matches = $state<FunctionMatch[]>([]);
  searchFailure = $state<string | null>(null);

  constructor(api: NeighbourhoodApi, projectId: () => string) {
    this.#api = api;
    this.#projectId = projectId;
  }

  setApi(api: NeighbourhoodApi) {
    this.#api = api;
  }

  get view(): 'pick' | 'loading' | 'error' | 'alone' | 'graph' {
    if (this.focus === null) return 'pick';
    if (this.loading) return 'loading';
    if (this.failure) return 'error';
    if (!this.payload || this.payload.edges.length === 0) return 'alone';
    return 'graph';
  }

  get focusCard(): NeighbourCard | null {
    return this.payload?.focus ?? null;
  }

  get nodes(): DrawnCard[] {
    return (this.payload?.nodes ?? []).map((n) => ({
      ...n,
      note: n.file ? (n.line ? `${n.file}:${n.line}` : n.file) : undefined,
    }));
  }

  get edges(): NeighbourEdge[] {
    return this.payload?.edges ?? [];
  }

  /** What the picture cannot show, one fact per line. */
  get coverageNotes(): string[] {
    const c = this.payload?.coverage;
    const name = this.focusCard?.label ?? 'this symbol';
    if (!c) return [];
    const notes: string[] = [];
    if (c.cut.in > 0) {
      notes.push(`${plural(c.cut.in, 'more caller', 'more callers')} found and not drawn — the busiest are shown.`);
    }
    if (c.cut.out > 0) {
      notes.push(`${plural(c.cut.out, 'more callee', 'more callees')} found and not drawn — the busiest are shown.`);
    }
    if (c.unplacedCallees > 0) {
      notes.push(
        `${plural(c.unplacedCallees, 'call', 'calls')} from ${name} could not be placed, so ${c.unplacedCallees === 1 ? 'it is' : 'they are'} not on the right.`,
      );
    }
    if (c.namedUnplaced > 0) {
      notes.push(
        `${plural(c.namedUnplaced, 'unplaced call uses', 'unplaced calls use')} the name ${name} — some may be calls to it, so the callers shown are a floor.`,
      );
    }
    if (notes.length === 0) notes.push('Every call to and from this symbol was placed and is drawn.');
    return notes;
  }

  async load() {
    const focus = this.focus;
    if (focus === null) return;
    const ticket = ++this.#ticket;
    this.loading = true;
    this.failure = null;
    const res = await this.#api.tryGetProjectNeighbourhood(this.#projectId(), focus, this.depth);
    if (ticket !== this.#ticket) return;
    if (res.ok) {
      this.payload = res.data;
    } else {
      this.payload = null;
      this.failure = failureText(
        res.error,
        'Not a symbol of this project — it may have been re-indexed away. Pick another.',
      );
    }
    this.loading = false;
  }

  async setFocus(id: string) {
    this.focus = id;
    this.refusal = null;
    await this.load();
  }

  /** Centre on a card already drawn — the way a reader keeps walking. */
  async recentre(id: string) {
    if (id === this.focus) return;
    const target = this.payload?.nodes.find((n) => n.id === id);
    if (target?.external) {
      this.refusal = `${target.label} is outside this project — its callers are not this picture.`;
      return;
    }
    await this.setFocus(id);
  }

  async setDepth(depth: number) {
    if (depth < MIN_DEPTH || depth > MAX_DEPTH || depth === this.depth) return;
    this.depth = depth;
    await this.load();
  }

  async search(query: string) {
    this.query = query;
    this.searchFailure = null;
    if (query.trim().length < MIN_QUERY) {
      this.matches = [];
      return;
    }
    const res = await this.#api.trySearchFunctions(this.#projectId(), query.trim());
    if (query !== this.query) return;
    if (res.ok) {
      this.matches = res.data;
    } else {
      this.matches = [];
      this.searchFailure = failureText(res.error, 'This project is not known to the daemon.');
    }
  }
}
