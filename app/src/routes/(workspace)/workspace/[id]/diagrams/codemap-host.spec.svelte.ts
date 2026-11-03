import { describe, expect, it } from 'vitest';
import { flushSync } from 'svelte';
import { CodeMapHost } from './codemap-host.svelte.js';
import type { CodeMapPort } from '@sensei/codemap';

describe('CodeMapHost', () => {
  // SvelteKit replaces `page.params` on every navigation, the same id included: a
  // lens switch must keep the map, and only another workspace gets a new one.
  it('keeps one map while the workspace id stays, and a new one for another', () => {
    const page = $state({ params: { id: 'w1' } });
    const host = new CodeMapHost(() => page.params.id, {} as CodeMapPort);
    const first = host.state;
    page.params = { id: 'w1' };
    flushSync();
    expect(host.state).toBe(first);
    page.params = { id: 'w2' };
    flushSync();
    expect(host.state).not.toBe(first);
  });

  it('gives the header nothing to show until the map is read, then its numbers, budget and level', () => {
    const host = new CodeMapHost(() => 'w', {} as CodeMapPort);
    expect([host.stats, host.budget, host.level]).toEqual([null, null, 'L0 · modules']);
    host.state.l0 = {
      level: 'module', groupBy: 'layers', cycles: [], coverage: { unplaced: 0 },
      blocks: [], bands: [], budget: { drawn: 0, modules: 0, bands: 0, edges: 0 }, libraries: { count: 0, top: [] },
    };
    expect(host.stats?.[0]).toMatchObject({ label: 'Modules', value: '0' });
    expect(host.budget).toBe('drawing 0 of 0 modules · 0 bands for 0 edges · 0 symbols');
  });
});
