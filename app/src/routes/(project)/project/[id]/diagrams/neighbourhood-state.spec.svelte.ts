import { describe, expect, it } from 'vitest';
import { NeighbourhoodState, type NeighbourhoodApi } from './neighbourhood-state.svelte.js';
import type {
  FunctionMatch,
  NeighbourCard,
  NeighbourCoverage,
  NeighbourhoodPayload,
} from '$lib/types.js';

function card(id: string, over: Partial<NeighbourCard> = {}): NeighbourCard {
  return {
    id,
    label: id,
    kind: 'function',
    group: 'senseid',
    module: 'api',
    language: 'rust',
    file: `src/${id}.rs`,
    line: 10,
    external: false,
    rows: [],
    ...over,
  };
}

function coverage(over: Partial<NeighbourCoverage> = {}): NeighbourCoverage {
  return { drawn: 2, cut: { in: 0, out: 0 }, unplacedCallees: 0, namedUnplaced: 0, ...over };
}

/** A caller, the focus, a callee and a library callee. */
function payload(over: Partial<NeighbourhoodPayload> = {}): NeighbourhoodPayload {
  return {
    depth: 1,
    focus: card('focus'),
    nodes: [card('caller'), card('focus'), card('callee'), card('to_string', { external: true, group: 'serde' })],
    edges: [
      { id: 'caller→focus', source: 'caller', target: 'focus', kind: 'dependency', weight: 1 },
      { id: 'focus→callee', source: 'focus', target: 'callee', kind: 'dependency', weight: 2 },
      { id: 'focus→to_string', source: 'focus', target: 'to_string', kind: 'dependency', weight: 1 },
    ],
    coverage: coverage(),
    ...over,
  };
}

type Call = { focus: string; depth: number };

function api(
  respond: (c: Call) => NeighbourhoodPayload | { status: number; message: string } = () => payload(),
  search: (q: string) => FunctionMatch[] | { status: number; message: string } = () => [],
) {
  const calls: Call[] = [];
  const searches: string[] = [];
  const a: NeighbourhoodApi = {
    tryGetProjectNeighbourhood: async (_id, focus, depth) => {
      calls.push({ focus, depth });
      const r = respond({ focus, depth });
      return 'status' in r ? { ok: false, error: r } : { ok: true, data: r };
    },
    trySearchFunctions: async (_id, q) => {
      searches.push(q);
      const r = search(q);
      return 'status' in r ? { ok: false, error: r } : { ok: true, data: r };
    },
  };
  return { a, calls, searches };
}

describe('NeighbourhoodState', () => {
  // There is no default symbol. Picking one would draw a neighbourhood the
  // reader did not ask about, so the screen opens on the picker and asks.
  it('with no focus it asks for one, and requests nothing', async () => {
    const { a, calls } = api();
    const s = new NeighbourhoodState(a, () => 'p1');
    await s.load();
    expect(s.view).toBe('pick');
    expect(calls).toEqual([]);
  });

  it('a focus loads its neighbourhood at the chosen depth', async () => {
    const { a, calls } = api();
    const s = new NeighbourhoodState(a, () => 'p1');
    await s.setFocus('focus');
    expect(calls).toEqual([{ focus: 'focus', depth: 1 }]);
    expect(s.view).toBe('graph');
    expect(s.focusCard?.label).toBe('focus');
  });

  it('a failed request is an error, never an empty picture', async () => {
    const { a } = api(() => ({ status: 500, message: 'Internal Server Error' }));
    const s = new NeighbourhoodState(a, () => 'p1');
    await s.setFocus('focus');
    expect(s.view).toBe('error');
    expect(s.failure).toContain('500');
    expect(s.nodes).toEqual([]);
  });

  it('a focus that is not this project’s says so, rather than looking unreachable', async () => {
    const { a } = api(() => ({ status: 404, message: 'Not Found' }));
    const s = new NeighbourhoodState(a, () => 'p1');
    await s.setFocus('elsewhere');
    expect(s.view).toBe('error');
    expect(s.failure).toMatch(/not a symbol of this project/i);
  });

  it('changing depth re-asks the daemon, and only within one to three', async () => {
    const { a, calls } = api();
    const s = new NeighbourhoodState(a, () => 'p1');
    await s.setFocus('focus');
    await s.setDepth(2);
    expect(calls.at(-1)).toEqual({ focus: 'focus', depth: 2 });
    await s.setDepth(4);
    await s.setDepth(0);
    expect(s.depth).toBe(2);
    expect(calls).toHaveLength(2);
  });

  // A library card is drawn because the focus really calls it — but its callers
  // are every project on the machine, which is not this picture. Re-centring on
  // it is refused WITH A REASON, not silently ignored.
  it('re-centring on a library card is refused, with a reason', async () => {
    const { a, calls } = api();
    const s = new NeighbourhoodState(a, () => 'p1');
    await s.setFocus('focus');
    await s.recentre('to_string');
    expect(s.focus).toBe('focus');
    expect(calls).toHaveLength(1);
    expect(s.refusal).toMatch(/outside this project/i);

    await s.recentre('callee');
    expect(s.focus).toBe('callee');
    expect(s.refusal).toBeNull();
  });

  it('re-centring on the focus itself asks nothing new', async () => {
    const { a, calls } = api();
    const s = new NeighbourhoodState(a, () => 'p1');
    await s.setFocus('focus');
    await s.recentre('focus');
    expect(calls).toHaveLength(1);
  });

  // A slow answer for an OLD focus must not overwrite the picture of the new
  // one — the reader would see callers of a symbol they have walked away from.
  it('a stale response cannot overwrite a newer focus', async () => {
    let release: (() => void) | undefined;
    const a: NeighbourhoodApi = {
      tryGetProjectNeighbourhood: async (_id, focus) => {
        if (focus === 'slow') await new Promise<void>((r) => (release = r));
        return { ok: true, data: payload({ focus: card(focus) }) };
      },
      trySearchFunctions: async () => ({ ok: true, data: [] }),
    };
    const s = new NeighbourhoodState(a, () => 'p1');
    const first = s.setFocus('slow');
    await s.setFocus('fast');
    release?.();
    await first;
    expect(s.focusCard?.label).toBe('fast');
  });

  describe('what the picture cannot show', () => {
    it('every call placed, nothing cut: one line that says so', async () => {
      const { a } = api();
      const s = new NeighbourhoodState(a, () => 'p1');
      await s.setFocus('focus');
      expect(s.coverageNotes).toEqual(['Every call to and from this symbol was placed and is drawn.']);
    });

    it('cuts, unplaced callees and unplaced names are each said, never summed', async () => {
      const { a } = api(() =>
        payload({ coverage: coverage({ cut: { in: 1212, out: 3 }, unplacedCallees: 4, namedUnplaced: 9 }) }),
      );
      const s = new NeighbourhoodState(a, () => 'p1');
      await s.setFocus('focus');
      const notes = s.coverageNotes.join('\n');
      expect(notes).toContain('1,212 more callers');
      expect(notes).toContain('3 more callees');
      expect(notes).toContain('4 calls from focus could not be placed');
      expect(notes).toContain('9 unplaced calls use the name focus');
      expect(s.coverageNotes).toHaveLength(4);
    });

    it('one of each is singular', async () => {
      const { a } = api(() =>
        payload({ coverage: coverage({ cut: { in: 1, out: 0 }, unplacedCallees: 1, namedUnplaced: 1 }) }),
      );
      const s = new NeighbourhoodState(a, () => 'p1');
      await s.setFocus('focus');
      const notes = s.coverageNotes.join('\n');
      expect(notes).toContain('1 more caller ');
      expect(notes).toContain('1 call from focus could not');
      expect(notes).toContain('1 unplaced call uses');
    });
  });

  it('a symbol with no placed neighbours is a finding in words, not an empty canvas', async () => {
    const { a } = api(() => payload({ nodes: [card('focus')], edges: [], coverage: coverage({ drawn: 0 }) }));
    const s = new NeighbourhoodState(a, () => 'p1');
    await s.setFocus('focus');
    expect(s.view).toBe('alone');
  });

  it('cards carry file:line as their note, so a reader can find the code', async () => {
    const { a } = api();
    const s = new NeighbourhoodState(a, () => 'p1');
    await s.setFocus('focus');
    expect(s.nodes.find((n) => n.id === 'callee')?.note).toBe('src/callee.rs:10');
    expect(s.nodes.find((n) => n.id === 'focus')?.note).toBe('src/focus.rs:10');
  });

  describe('the picker', () => {
    it('does not search on fewer than two characters', async () => {
      const { a, searches } = api();
      const s = new NeighbourhoodState(a, () => 'p1');
      await s.search('a');
      expect(searches).toEqual([]);
      expect(s.matches).toEqual([]);
    });

    it('lists matches', async () => {
      const m: FunctionMatch = { id: 'n1', name: 'upsert_repo', file_path: 'a.rs', signature: null, line_start: 3 };
      const { a } = api(undefined, () => [m]);
      const s = new NeighbourhoodState(a, () => 'p1');
      await s.search('upsert');
      expect(s.matches).toEqual([m]);
      expect(s.searchFailure).toBeNull();
    });

    // "No matches" when the daemon is down sends the reader looking for a
    // symbol that exists.
    it('a failed search is a failure, not "no matches"', async () => {
      const { a } = api(undefined, () => ({ status: 0, message: 'connection refused' }));
      const s = new NeighbourhoodState(a, () => 'p1');
      await s.search('upsert');
      expect(s.searchFailure).toMatch(/could not reach the daemon/i);
      expect(s.matches).toEqual([]);
    });
  });
});
