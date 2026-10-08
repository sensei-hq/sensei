import { describe, expect, it } from 'vitest';
import { WORLD_FIELDS, WorldState, type WorldApi } from './world-state.svelte.js';
import type { WorldPayload, WorldUnit } from '$lib/types.js';

function unit(path: string[], weight: number, over: Partial<WorldUnit> = {}): WorldUnit {
  return {
    id: path.join(' / '),
    label: path[path.length - 1],
    path,
    weight,
    measures: { documentedShare: 0.1, testShare: 0.2, unresolvedShare: 0.3 },
    ...over,
  };
}

/** Two projects, one shared repository — the shape the corpus actually has. */
function payload(over: Partial<WorldPayload> = {}): WorldPayload {
  return {
    groupBy: 'project',
    viewing: 'alpha',
    units: [
      unit(['alpha'], 100),
      unit(['alpha', 'core'], 100),
      unit(['alpha', 'core', 'code'], 80, {
        measures: { documentedShare: 0.1, testShare: 0, unresolvedShare: null },
      }),
      unit(['alpha', 'core', 'tests'], 20, {
        measures: { documentedShare: 0, testShare: 1, unresolvedShare: null },
      }),
      unit(['beta'], 130),
      unit(['beta', 'core'], 80),
      unit(['beta', 'web'], 50),
    ],
    totals: { declarations: 230, documented: 41, tests: 20, repositories: 2, projects: 2 },
    ...over,
  };
}

function okApi(data: WorldPayload = payload()): WorldApi {
  return { tryGetProjectWorld: async () => ({ ok: true, data }) };
}

function failApi(status: number, message: string): WorldApi {
  return { tryGetProjectWorld: async () => ({ ok: false, error: { status, message } }) };
}

describe('WorldState', () => {
  it('starts in loading, before any request has settled', () => {
    expect(new WorldState(okApi(), () => 'p1').view).toBe('loading');
  });

  it('a failed request is an error, never an empty picture', async () => {
    const s = new WorldState(failApi(500, 'Internal Server Error'), () => 'p1');
    await s.load();
    expect(s.view).toBe('error');
    expect(s.units).toEqual([]);
  });

  // THE READING THIS SCREEN IS FOR. The picture is all indexed code, which is
  // wider than the project the reader opened — so it has to say which circle
  // they are standing in, or the view silently reads as "my project".
  it('names the circle the reader is standing in', async () => {
    const s = new WorldState(okApi(), () => 'p1');
    await s.load();
    expect(s.viewingNote).toContain('alpha');
    expect(s.viewingNote).toContain('2 projects');
  });

  // Drill-down is a PATH, not an id — that is what the layout takes, and a path
  // cannot dangle the way a parent id can.
  it('drilling in scopes the panel to the focused circle', async () => {
    const s = new WorldState(okApi(), () => 'p1');
    await s.load();
    expect(s.inView.label).toBe('All indexed code');
    expect(s.inView.weight).toBe(230);

    s.drillTo(['alpha', 'core']);
    expect(s.focusPath).toEqual(['alpha', 'core']);
    expect(s.inView.label).toBe('core');
    expect(s.inView.weight).toBe(100);

    s.drillUp();
    expect(s.focusPath).toEqual(['alpha']);
    expect(s.inView.weight).toBe(100);
  });

  // The root is a real place and `drillUp` from it must not leave the picture
  // in a path that names nothing.
  it('cannot drill above the root', async () => {
    const s = new WorldState(okApi(), () => 'p1');
    await s.load();
    s.drillUp();
    expect(s.focusPath).toEqual([]);
    expect(s.inView.label).toBe('All indexed code');
  });

  /** What sits directly inside the focused circle, biggest first — the
   *  mockup's "largest inside" list. */
  it('lists what is directly inside, biggest first, and nothing deeper', async () => {
    const s = new WorldState(okApi(), () => 'p1');
    await s.load();
    expect(s.largestInside.map((u) => u.label)).toEqual(['beta', 'alpha']);

    s.drillTo(['alpha']);
    expect(s.largestInside.map((u) => u.label)).toEqual(['core']);

    s.drillTo(['alpha', 'core']);
    // Children only — `code` and `tests`, not the grandchildren of anything.
    expect(s.largestInside.map((u) => u.label)).toEqual(['code', 'tests']);
  });

  // A grouping change invalidates a path built from the OLD ordering, so
  // keeping it would focus the picture on a circle that no longer exists.
  it('a grouping change drops a focus that names nothing at the new ordering', async () => {
    const s = new WorldState(okApi(), () => 'p1');
    await s.load();
    s.drillTo(['alpha', 'core']);
    s.setGroupBy('repository');
    expect(s.focusPath).toEqual([]);
    expect(s.groupBy).toBe('repository');
  });

  // `nothing` is the default and a first-class choice: the picture's first job
  // is size, and colouring before anyone asked turns a glance into a reading.
  it('shades by nothing until asked', async () => {
    const s = new WorldState(okApi(), () => 'p1');
    await s.load();
    expect(s.shadeBy).toBe('nothing');
    expect(s.shadeKey).toBeUndefined();
    s.setShadeBy('unresolvedShare');
    expect(s.shadeKey).toBe('unresolvedShare');
  });

  // THE MEASURE THAT IS NOT ALWAYS A FACT. Below the repository ring the
  // unresolved share is null, and the panel must say so rather than print a
  // zero that reads as "perfectly resolved".
  it('reports an absent measure as absent, never as zero', async () => {
    const s = new WorldState(okApi(), () => 'p1');
    await s.load();
    s.drillTo(['alpha', 'core', 'code']);
    expect(s.inView.measures.unresolvedShare).toBeNull();
    expect(s.measureNote('unresolvedShare')).toContain('not a fact at this level');

    s.drillTo(['alpha']);
    expect(s.measureNote('unresolvedShare')).toContain('30%');
  });

  it('states the corpus totals beside the picture', async () => {
    const s = new WorldState(okApi(), () => 'p1');
    await s.load();
    expect(s.totalsLine).toContain('230 declarations');
    expect(s.totalsLine).toContain('2 repositories');
  });
  // `GraphState.update` REPLACES the config — anything it is not given reverts
  // to the default. The page used to construct the state with `layout: 'world'`
  // and then update it with nodes/shade/focus alone, so the first update threw
  // the layout away and the "circle pack" rendered as a stack of cards (#219).
  // The config is therefore whole, every time.
  it('the graph config always carries the world layout and its sizing', async () => {
    const s = new WorldState(okApi(), () => 'p1');
    await s.load();
    s.setShadeBy('testShare');
    s.drillTo(['alpha']);
    const c = s.graphConfig;
    expect(c.layout).toBe('world');
    expect(c.sizeBy).toBe('weight');
    expect(c.sizeScale).toBe('log');
    expect(c.fields).toBe(WORLD_FIELDS);
    expect(c.shadeBy).toBe('testShare');
    expect(c.focusPath).toEqual(['alpha']);
    // Every unit is drawn; only container weights differ (see below).
    expect(c.nodes.map((n) => n.id)).toEqual(s.units.map((u) => u.id));
  });
  // rokkit's world layout treats a declared container's `weight` as its OWN
  // measure and SUMS its children on top. The payload's container weight is its
  // TOTAL (which the panel needs), so passing it through counted every
  // declaration once per ring: live, 27,295 declarations drew as "81.9K" — three
  // rings deep, three times over. The graph gets weight on LEAVES only.
  it('what the layout sums is the corpus, counted once', async () => {
    const s = new WorldState(okApi(), () => 'p1');
    await s.load();
    const nodes = s.graphConfig.nodes;
    const paths = nodes.map((n) => n.path.join('/'));
    const isLeaf = (p: string) => !paths.some((q) => q.startsWith(p + '/'));
    // What summarise() does: each node's own weight, summed over the tree.
    const drawn = nodes.reduce((sum, n) => sum + n.weight, 0);
    expect(drawn).toBe(230);
    for (const n of nodes) {
      if (!isLeaf(n.path.join('/'))) expect(n.weight, n.path.join('/')).toBe(0);
    }
    // The panel still reads totals.
    expect(s.units.find((u) => u.path.join('/') === 'alpha')?.weight).toBe(100);
  });
  // A CALLER-SUPPLIED GraphState takes its callbacks from its own config; the
  // `ondrill` props on `<Graph>` are wired only into the state Graph owns. So a
  // drill on the canvas moved the picture and left the IN VIEW panel standing at
  // the root (#219, seen live). The config carries the callbacks.
  it('a drill on the canvas moves the panel with it', async () => {
    const s = new WorldState(okApi(), () => 'p1');
    await s.load();
    s.graphConfig.ondrill?.(['alpha', 'core']);
    expect(s.inView.label).toBe('core');
    s.graphConfig.ondrillup?.(['alpha']);
    expect(s.focusPath).toEqual(['alpha']);
    s.graphConfig.onfocuspath?.([]);
    expect(s.inView.label).toBe('All indexed code');
  });
});
