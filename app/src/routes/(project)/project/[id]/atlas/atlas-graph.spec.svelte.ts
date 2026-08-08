import { describe, it, expect } from 'vitest';
import {
  parseCommunityLabel,
  buildCommunities,
  buildSymbolGraph,
  neighbourIds,
  scaleRadius,
  kindColor,
  kindNeedsRing,
  legendItems,
  initialLevel,
  buildAtlasPage,
  SYMBOL_CAP,
  COLLAPSE_THRESHOLD,
} from './atlas-graph.svelte.js';
import type { GraphSymbolNode, GraphCallEdge, CommunityInfo } from '$lib/types.js';

const node = (id: string, over: Partial<GraphSymbolNode> = {}): GraphSymbolNode => ({
  id,
  kind: 'function',
  name: id,
  file_path: 'src/x.rs',
  ...over,
});
const edge = (id: string, source: string, target: string | null, name?: string): GraphCallEdge => ({
  id,
  source_id: source,
  target_id: target,
  target_name: name ?? null,
});

describe('parseCommunityLabel', () => {
  it('splits "{kind} ({path})" into kind + path', () => {
    expect(parseCommunityLabel('function (app/src-tauri/src/commands)')).toEqual({
      kind: 'function',
      path: 'app/src-tauri/src/commands',
    });
  });

  it('handles a nested/parenthesised path', () => {
    expect(parseCommunityLabel('function (app/src/routes/(observatory))')).toEqual({
      kind: 'function',
      path: 'app/src/routes/(observatory)',
    });
  });

  it('falls back to the whole string when it has no parens', () => {
    expect(parseCommunityLabel('orphans')).toEqual({ kind: 'orphans', path: '' });
  });
});

describe('buildCommunities', () => {
  it('maps the wire shape and sorts largest first', () => {
    const raw: CommunityInfo[] = [
      { id: 'a', label: 'file (app/src)', node_count: 35 },
      { id: 'b', label: 'function (app/src-tauri/src/commands)', node_count: 530 },
    ];
    const out = buildCommunities(raw);
    expect(out.map((c) => c.id)).toEqual(['b', 'a']);
    expect(out[0]).toEqual({
      id: 'b',
      kind: 'function',
      path: 'app/src-tauri/src/commands',
      label: 'function (app/src-tauri/src/commands)',
      nodeCount: 530,
    });
  });
});

describe('buildSymbolGraph', () => {
  const nodes = [node('a'), node('b'), node('c'), node('d')];
  // a→b, a→c internal; a→Ok (target_id null) and a→x (out of scope) dropped.
  const edges = [
    edge('e1', 'a', 'b'),
    edge('e2', 'a', 'c'),
    edge('e3', 'a', null, 'Ok'),
    edge('e4', 'a', 'zzz'),
  ];

  it('drops unresolved + out-of-scope edges and counts internal degree', () => {
    const g = buildSymbolGraph(nodes, edges);
    expect(g.totalInternalEdges).toBe(2);
    expect(g.totalSymbols).toBe(4);
    const a = g.symbols.find((s) => s.id === 'a');
    expect(a?.degree).toBe(2);
  });

  it('ranks by degree and caps, keeping only links among kept symbols', () => {
    const g = buildSymbolGraph(nodes, edges, 2);
    // a (deg 2) then one of b/c (deg 1). d (deg 0) is dropped by the cap.
    expect(g.symbols).toHaveLength(2);
    expect(g.symbols[0].id).toBe('a');
    // Only links whose BOTH endpoints survive the cap are returned.
    for (const l of g.links) {
      const ids = g.symbols.map((s) => s.id);
      expect(ids).toContain(l.source);
      expect(ids).toContain(l.target);
    }
  });

  it('returns an empty graph for empty input', () => {
    const g = buildSymbolGraph([], []);
    expect(g.symbols).toEqual([]);
    expect(g.links).toEqual([]);
    expect(g.totalInternalEdges).toBe(0);
  });
});

describe('neighbourIds', () => {
  const links = [
    { source: 'a', target: 'b' },
    { source: 'c', target: 'a' },
    { source: 'b', target: 'd' },
  ];

  it('returns the node plus its direct neighbours (both directions)', () => {
    expect([...neighbourIds(links, 'a')].sort()).toEqual(['a', 'b', 'c']);
  });

  it('returns an empty set when nothing is selected', () => {
    expect(neighbourIds(links, null).size).toBe(0);
  });
});

describe('scaleRadius', () => {
  it('maps 0 to the minimum and the max value to the maximum', () => {
    expect(scaleRadius(0, 100, 5, 40)).toBe(5);
    expect(scaleRadius(100, 100, 5, 40)).toBe(40);
  });

  it('uses a sqrt (area) scale between the ends', () => {
    // sqrt(0.25) = 0.5 → midpoint of the range
    expect(scaleRadius(25, 100, 0, 40)).toBeCloseTo(20);
  });

  it('returns the minimum when the max is zero', () => {
    expect(scaleRadius(0, 0, 6, 40)).toBe(6);
  });
});

describe('kindColor / kindNeedsRing', () => {
  it('maps functions to accent and types to ink', () => {
    expect(kindColor('function')).toBe('var(--accent)');
    expect(kindColor('class')).toBe('var(--ink)');
  });

  it('falls back to a muted ink for unknown kinds', () => {
    expect(kindColor('mystery')).toBe('var(--ink-mute)');
  });

  it('rings only the low-contrast file/module fills', () => {
    expect(kindNeedsRing('file')).toBe(true);
    expect(kindNeedsRing('module')).toBe(true);
    expect(kindNeedsRing('function')).toBe(false);
  });
});

describe('legendItems', () => {
  it('emits one entry per present kind in canonical order', () => {
    const items = legendItems(['module', 'function', 'class', 'function']);
    expect(items.map((i) => i.kind)).toEqual(['function', 'class', 'module']);
    expect(items[0]).toEqual({ kind: 'function', color: 'var(--accent)', ring: false });
    expect(items.find((i) => i.kind === 'module')?.ring).toBe(true);
  });

  it('appends unknown kinds alphabetically after the known ones', () => {
    const items = legendItems(['zeta', 'function', 'alpha']);
    expect(items.map((i) => i.kind)).toEqual(['function', 'alpha', 'zeta']);
  });
});

describe('initialLevel', () => {
  it('opens on symbols for a small graph', () => {
    expect(initialLevel(COLLAPSE_THRESHOLD, 10)).toBe('symbols');
  });

  it('auto-collapses to communities past the threshold', () => {
    expect(initialLevel(COLLAPSE_THRESHOLD + 1, 10)).toBe('communities');
  });

  it('falls back to symbols when there are no communities', () => {
    expect(initialLevel(9000, 0)).toBe('symbols');
  });
});

describe('buildAtlasPage', () => {
  // The project's repo roots (graph is keyed by repo name); the loader passes the
  // project's own scopes, not every project.
  const scopes = [
    { id: 'p2', name: 'zeta' },
    { id: 'p1', name: 'alpha' },
  ];

  it('assembles a compact payload from the four raw responses', () => {
    const page = buildAtlasPage({
      repoId: 'sensei',
      scopes,
      communities: [
        { id: 'c1', label: 'function (a)', node_count: 10 },
        { id: 'c2', label: 'file (b)', node_count: 5 },
      ],
      callFlow: { moduleCount: 3, exportCount: 7, callCount: 42 },
      graph: { nodes: [node('a'), node('b')], edges: [edge('e1', 'a', 'b')] },
      solution: { repos: 2, nodes: 100, edges: 50 },
    });

    expect(page.repoId).toBe('sensei');
    expect(page.scopes.map((s) => s.name)).toEqual(['alpha', 'zeta']); // sorted
    expect(page.communityNodeTotal).toBe(15);
    expect(page.stats).toEqual({ modules: 3, exports: 7, calls: 42 });
    expect(page.solution).toEqual({ repos: 2, nodes: 100, edges: 50 });
    expect(page.totalSymbols).toBe(2);
    expect(page.cap).toBe(SYMBOL_CAP);
  });

  it('defaults the solution roll-up to zeros when absent', () => {
    const page = buildAtlasPage({
      repoId: 'x',
      scopes: [],
      communities: [],
      callFlow: { moduleCount: 0, exportCount: 0, callCount: 0 },
      graph: { nodes: [], edges: [] },
      solution: null,
    });
    expect(page.solution).toEqual({ repos: 0, nodes: 0, edges: 0 });
  });

  it('builds a production-only `code` view that excludes test-file nodes', () => {
    const page = buildAtlasPage({
      repoId: 'sensei',
      scopes,
      communities: [],
      callFlow: { moduleCount: 0, exportCount: 0, callCount: 0 },
      graph: {
        nodes: [
          node('prod', { file_path: 'src/lib.rs' }),
          node('helper', { file_path: 'src/lib.rs' }),
          node('t1', { file_path: 'tests/it.rs', is_test: true }),
          node('t2', { file_path: 'tests/it.rs', is_test: true }),
        ],
        edges: [edge('e1', 'prod', 'helper'), edge('e2', 't1', 't2')],
      },
      solution: null,
    });
    // Full view keeps all 4; the code view drops both test nodes BEFORE the cap.
    expect(page.totalSymbols).toBe(4);
    expect(page.code.totalSymbols).toBe(2);
    expect(page.code.symbols.map((s) => s.id).sort()).toEqual(['helper', 'prod']);
    // The test-only edge (t1→t2) is gone; the production edge survives.
    expect(page.code.links).toEqual([{ source: 'prod', target: 'helper' }]);
  });
});
