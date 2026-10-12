import { describe, expect, it } from 'vitest';
import { LayersState, type LayeringApi } from './layering-state.svelte.js';
import type { LayeringPayload } from '$lib/types.js';

/** `sensei`'s own shape, shrunk: `db` and `tasks` close a cycle, `tasks`
 *  depends on `util` below it, and `db → tasks` is the weakest link. */
function payload(over: Partial<LayeringPayload> = {}): LayeringPayload {
  return {
    level: 'module',
    kinds: ['calls'],
    layerSource: 'derived',
    depth: 2,
    nodes: [
      {
        id: 'senseid/db',
        label: 'db',
        group: 'senseid',
        weight: 40,
        files: 9,
        language: 'rust',
        layer: 0,
        component: 0,
        componentSize: 2,
      },
      {
        id: 'senseid/tasks',
        label: 'tasks',
        group: 'senseid',
        weight: 30,
        files: 7,
        language: 'rust',
        layer: 0,
        component: 0,
        componentSize: 2,
      },
      {
        id: 'senseid/util',
        label: 'util',
        group: 'senseid',
        weight: 5,
        files: 1,
        language: 'rust',
        layer: 1,
        component: 1,
        componentSize: 1,
      },
    ],
    edges: [
      {
        id: 'senseid/db→senseid/tasks',
        source: 'senseid/db',
        target: 'senseid/tasks',
        kind: 'dependency',
        weight: 149,
        conformance: 'up',
        weakest: true,
      },
      {
        id: 'senseid/tasks→senseid/db',
        source: 'senseid/tasks',
        target: 'senseid/db',
        kind: 'dependency',
        weight: 200,
        conformance: 'level',
        weakest: false,
      },
      {
        id: 'senseid/tasks→senseid/util',
        source: 'senseid/tasks',
        target: 'senseid/util',
        kind: 'dependency',
        weight: 12,
        conformance: 'down',
        weakest: false,
      },
    ],
    cycles: [
      {
        component: 0,
        members: ['senseid/db', 'senseid/tasks'],
        layer: 0,
        inner: [
          { source: 'senseid/tasks', target: 'senseid/db', occurrences: 200 },
          { source: 'senseid/db', target: 'senseid/tasks', occurrences: 149 },
        ],
        cut: { source: 'senseid/db', target: 'senseid/tasks', occurrences: 149 },
      },
    ],
    selfDependencies: [{ source: 'senseid/db', target: 'senseid/db', occurrences: 4 }],
    coverage: { drawn: 3, unplaced: 41, units: 3, unknownUnit: 32 },
    ...over,
  };
}

function okApi(data: LayeringPayload = payload()): LayeringApi {
  return { tryGetProjectLayering: async () => ({ ok: true, data }) };
}

function failApi(status: number, message: string): LayeringApi {
  return { tryGetProjectLayering: async () => ({ ok: false, error: { status, message } }) };
}

describe('LayersState', () => {
  it('starts in loading, before any request has settled', () => {
    expect(new LayersState(okApi(), () => 'p1').view).toBe('loading');
  });

  // The same defect the Structure screen is specified against, and it bites
  // harder here: an empty Layers canvas reads as a clean architecture.
  it('a failed request is an error, never an empty graph', async () => {
    const s = new LayersState(failApi(500, 'Internal Server Error'), () => 'p1');
    await s.load();
    expect(s.view).toBe('error');
    expect(s.nodes).toEqual([]);
    expect(s.failure).toContain('500');
  });

  it('keeps a previous payload out of an error render', async () => {
    const s = new LayersState(okApi(), () => 'p1');
    await s.load();
    expect(s.view).toBe('graph');
    s.setApi(failApi(503, 'Service Unavailable'));
    await s.load();
    expect(s.nodes).toEqual([]);
  });

  // THE ONE THAT MATTERS. `skip` is legal under relaxed layering and rokkit's
  // own vocabulary separates it from a climb, so a screen that counts both
  // reports a problem the architecture does not have.
  it('counts only climbs as breaks — a skip is not one', async () => {
    const s = new LayersState(
      okApi(
        payload({
          edges: [
            ...payload().edges,
            {
              id: 'a→b',
              source: 'senseid/db',
              target: 'senseid/util',
              kind: 'dependency',
              weight: 3,
              conformance: 'skip',
              weakest: false,
            },
          ],
        }),
      ),
      () => 'p1',
    );
    await s.load();
    expect(s.breaks).toHaveLength(1);
    expect(s.breaks[0].conformance).toBe('up');
  });

  // `unknownUnit` is the number the issue says must sit beside the picture.
  it('states what the picture omits, including the unit it could not name', async () => {
    const s = new LayersState(okApi(), () => 'p1');
    await s.load();
    expect(s.coverageLine).toContain('3 units');
    expect(s.coverageLine).toContain('3 dependencies drawn');
    expect(s.coverageLine).toContain('32 with an endpoint that owns no file');
  });

  // Derived, not declared — so the screen answers a narrower question than
  // "is this the architecture you intended", and has to say so.
  it('says the layering was measured rather than declared', async () => {
    const s = new LayersState(okApi(), () => 'p1');
    await s.load();
    expect(s.sourceNote).toContain('measured');
  });

  // Zero breaks is a READING, not an absence: it has to be distinguishable
  // from "nothing loaded".
  it('reports a clean downward flow as a finding of its own', async () => {
    const clean = payload({
      edges: payload().edges.map((e) => ({ ...e, conformance: 'down' as const })),
      cycles: [],
    });
    const s = new LayersState(okApi(clean), () => 'p1');
    await s.load();
    expect(s.view).toBe('graph');
    expect(s.breaks).toEqual([]);
    expect(s.breaksNote).toContain('No call climbs');
  });

  it('a level change drops a selection that names nothing at the new grain', async () => {
    const s = new LayersState(okApi(), () => 'p1');
    await s.load();
    s.selected = 'senseid/db';
    s.setLevel('file');
    expect(s.selected).toBeNull();
    expect(s.level).toBe('file');
  });

  it('refuses to turn the last edge kind off, because the endpoint 400s on none', async () => {
    const s = new LayersState(okApi(), () => 'p1');
    s.toggleKind('calls');
    expect(s.kinds).toEqual(['calls']);
  });
});

describe('LayersState · cycles', () => {
  // The condensation reads top-down, so the deepest-depended-on cycle first.
  it('orders cycles by layer, then by size', async () => {
    const two = payload({
      cycles: [
        { component: 3, members: ['x', 'y'], layer: 2, inner: [], cut: null },
        { component: 0, members: ['a', 'b', 'c'], layer: 0, inner: [], cut: null },
      ],
    });
    const s = new LayersState(okApi(two), () => 'p1');
    await s.load();
    expect(s.cycles.map((c) => c.component)).toEqual([0, 3]);
  });

  /** The sentence the mockup's list row reads. Built here rather than in the
   *  template so it is asserted once and cannot drift between the two screens. */
  it('names the cut and what it opens', async () => {
    const s = new LayersState(okApi(), () => 'p1');
    await s.load();
    expect(s.cutLine(s.cycles[0])).toBe(
      'Cut senseid/db → senseid/tasks (×149, the weakest link) and the loop opens.',
    );
  });

  // A component with no cuttable edge is a real state, and saying nothing
  // about it would read as a cycle that needs no action.
  it('says so when a cycle has no weakest link to cut', async () => {
    const s = new LayersState(
      okApi(payload({ cycles: [{ ...payload().cycles[0], cut: null }] })),
      () => 'p1',
    );
    await s.load();
    expect(s.cutLine(s.cycles[0])).toContain('No single dependency');
  });

  it('counts a unit depending on itself separately from a cycle', async () => {
    const s = new LayersState(okApi(), () => 'p1');
    await s.load();
    expect(s.selfDependencies).toHaveLength(1);
    expect(s.cycles).toHaveLength(1);
  });
});
