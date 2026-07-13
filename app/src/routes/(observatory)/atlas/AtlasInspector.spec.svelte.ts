// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import AtlasInspectorHarness from './AtlasInspector.harness.svelte';
import type { InspectorOverview } from './atlas-graph.svelte.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const overview = (over: Partial<InspectorOverview> = {}): InspectorOverview => ({
  nodes: 79,
  relations: 0,
  communities: 79,
  ...over,
});

describe('AtlasInspector', () => {
  it('shows the view roll-up when nothing is selected', () => {
    const m = mountComponent(AtlasInspectorHarness, { overview: overview() });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('This view');
    expect(m.container.textContent).toContain('79');
    expect(m.container.querySelector('[data-atlas-cap]')).toBeNull();
  });

  it('surfaces the honest "showing N of M" cap line in the symbol view', () => {
    const m = mountComponent(AtlasInspectorHarness, {
      overview: overview({ nodes: 200, relations: 120, shown: 200, total: 6598 }),
    });
    cleanup.push(m.destroy);
    const cap = m.container.querySelector('[data-atlas-cap]');
    expect(cap).not.toBeNull();
    expect(cap!.textContent).toContain('200');
    expect(cap!.textContent).toContain('6598');
  });

  it('renders a focused community with its share', () => {
    const m = mountComponent(AtlasInspectorHarness, {
      overview: overview(),
      community: {
        label: 'function (crates/senseid/src/db)',
        kind: 'method',
        path: 'crates/senseid/src/db',
        nodeCount: 241,
        sharePct: 12,
      },
    });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('crates/senseid/src/db');
    expect(m.container.textContent).toContain('241');
    expect(m.container.textContent).toContain('12%');
  });

  it('renders a focused symbol with its calls and callers', () => {
    const m = mountComponent(AtlasInspectorHarness, {
      overview: overview(),
      symbol: { name: 'get_repo_by_name', kind: 'function', file: 'crates/senseid/src/db/pg_store.rs', degree: 5 },
      dependsOn: ['scope_folder_ids', 'json_uuid'],
      usedBy: ['community_info', 'call_flow'],
    });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('get_repo_by_name');
    expect(m.container.textContent).toContain('pg_store.rs');
    expect(m.container.textContent).toContain('Calls · 2');
    expect(m.container.textContent).toContain('scope_folder_ids');
    expect(m.container.textContent).toContain('Called by · 2');
    expect(m.container.textContent).toContain('community_info');
  });
});
