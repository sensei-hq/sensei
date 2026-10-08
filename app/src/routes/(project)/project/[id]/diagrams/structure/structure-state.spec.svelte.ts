import { describe, expect, it } from 'vitest';
import { StructureState, type StructureApi } from './structure-state.svelte.js';
import type { StructureLevel, StructurePayload } from '$lib/types.js';

function payload(over: Partial<StructurePayload> = {}): StructurePayload {
  return {
    level: 'file',
    kinds: ['calls'],
    nodes: [
      {
        id: 'a.rs',
        label: 'a.rs',
        package: 'pkg',
        module: 'tasks',
        language: 'rust',
        path: ['pkg', 'tasks', 'a.rs'],
        files: 1,
        symbols: 7,
      },
      {
        id: 'b.rs',
        label: 'b.rs',
        package: 'pkg',
        module: 'api',
        language: 'rust',
        path: ['pkg', 'api', 'b.rs'],
        files: 1,
        symbols: 3,
      },
    ],
    edges: [
      { source: 'a.rs', target: 'b.rs', kind: 'calls', span: 'cross_module', occurrences: 2 },
      { source: 'b.rs', target: 'a.rs', kind: 'calls', span: 'in_module', occurrences: 1 },
    ],
    coverage: { drawn: 3, unplaced: 41 },
    ...over,
  };
}

function okApi(data: StructurePayload = payload()): StructureApi {
  return { tryGetProjectStructure: async () => ({ ok: true, data }) };
}

function failApi(status: number, message: string): StructureApi {
  return { tryGetProjectStructure: async () => ({ ok: false, error: { status, message } }) };
}

describe('StructureState', () => {
  it('starts in loading, before any request has settled', () => {
    const s = new StructureState(okApi(), () => 'p1');
    expect(s.view).toBe('loading');
  });

  // The defect this screen is specified against. A failed request must NOT
  // reach the same render as a project with nothing in it.
  it('a failed request is an error, never an empty graph', async () => {
    const s = new StructureState(failApi(500, 'Internal Server Error'), () => 'p1');
    await s.load();
    expect(s.view).toBe('error');
    expect(s.nodes).toEqual([]);
    expect(s.failure).toContain('500');
  });

  it('an unreachable daemon says so, rather than reporting a server status', async () => {
    const s = new StructureState(failApi(0, 'Network error'), () => 'p1');
    await s.load();
    expect(s.view).toBe('error');
    expect(s.failure).toContain('Could not reach the daemon');
  });

  it('a successful request with no nodes is empty, and that is distinct', async () => {
    const s = new StructureState(okApi(payload({ nodes: [], edges: [] })), () => 'p1');
    await s.load();
    expect(s.view).toBe('empty');
    expect(s.failure).toBeNull();
  });

  // A stale graph under an error banner shows something the daemon did not say.
  it('drops a previous payload when a later request fails', async () => {
    let fail = false;
    const api: StructureApi = {
      tryGetProjectStructure: async () =>
        fail
          ? { ok: false, error: { status: 500, message: 'boom' } }
          : { ok: true, data: payload() },
    };
    const s = new StructureState(api, () => 'p1');
    await s.load();
    expect(s.nodes).toHaveLength(2);
    fail = true;
    await s.load();
    expect(s.view).toBe('error');
    expect(s.nodes).toEqual([]);
  });

  it('states the unplaced count even when the graph is drawn', async () => {
    const s = new StructureState(okApi(), () => 'p1');
    await s.load();
    expect(s.coverageLine).toBe('2 nodes · 2 edges drawn · 41 unplaced (not shown)');
  });

  // Counted over what is DRAWN, and `crossing` excludes in-module edges.
  it('degree counts the drawn edges, separating the crossing ones', async () => {
    const s = new StructureState(okApi(), () => 'p1');
    await s.load();
    s.selected = 'a.rs';
    expect(s.degree).toEqual({ inbound: 1, outbound: 1, crossing: 1 });
    s.selected = 'b.rs';
    expect(s.degree).toEqual({ inbound: 1, outbound: 1, crossing: 0 });
  });

  it('clears a selection when the level changes, because the id names nothing there', async () => {
    const s = new StructureState(okApi(), () => 'p1');
    await s.load();
    s.selected = 'a.rs';
    s.setLevel('module' as StructureLevel);
    expect(s.selected).toBeNull();
    expect(s.level).toBe('module');
  });

  // The endpoint 400s on an empty kind list. Turning the last one off is a
  // no-op rather than a request that fails.
  it('refuses to turn off the last edge kind', () => {
    const s = new StructureState(okApi(), () => 'p1');
    expect(s.kinds).toEqual(['calls']);
    s.toggleKind('calls');
    expect(s.kinds).toEqual(['calls']);
    s.toggleKind('imports');
    expect(s.kinds).toEqual(['calls', 'imports']);
    s.toggleKind('calls');
    expect(s.kinds).toEqual(['imports']);
  });

  it('passes the level and kinds the controls hold to the daemon', async () => {
    const seen: Array<[string, string, string[]]> = [];
    const api: StructureApi = {
      tryGetProjectStructure: async (id, level, kinds) => {
        seen.push([id, level, [...kinds]]);
        return { ok: true, data: payload() };
      },
    };
    const s = new StructureState(api, () => 'proj-7');
    s.setLevel('package' as StructureLevel);
    s.toggleKind('imports');
    await s.load();
    expect(seen).toEqual([['proj-7', 'package', ['calls', 'imports']]]);
  });
});
