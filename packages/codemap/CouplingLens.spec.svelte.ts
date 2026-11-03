// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import CouplingLens from './CouplingLens.svelte';
import type { CoChange } from './types';

let app: Record<string, unknown> | null = null;
let target: HTMLElement;
afterEach(() => { if (app) unmount(app); app = null; target?.remove(); });

const CC: CoChange = {
  hidden: [
    { a: 'p/ui', b: 'p/api', together: 6, strength: 0.4 },
    { a: 'p/db', b: 'p/cache', together: 11, strength: 0.8 },
  ],
  pairs: 9, commits: 120, minTogether: 5,
};

function render(data: CoChange | null = CC) {
  const onopen = vi.fn();
  target = document.body.appendChild(document.createElement('div'));
  app = mount(CouplingLens, { target, props: { data, zoom: 1, onopen } });
  flushSync();
  return onopen;
}

// Hidden coupling is its own facet (user, 2026-11-02): only the pairs that change
// together with no dependency between them, strongest first — not the block map.
describe('CouplingLens', () => {
  it('a row per hidden pair, strongest first, saying how often they change together', () => {
    render();
    const rows = [...target.querySelectorAll('[data-pair]')];
    expect(rows.map((r) => r.getAttribute('data-pair'))).toEqual(['p/db|p/cache', 'p/ui|p/api']);
    expect(rows[0].textContent).toContain('11 commits');
    expect(rows[0].textContent).toContain('80%');
  });

  it('either side opens its module', () => {
    const onopen = render();
    (target.querySelector('[data-pair="p/ui|p/api"] button[aria-label="Open p/api"]') as HTMLButtonElement).click();
    expect(onopen).toHaveBeenCalledWith('p/api');
  });

  it('tells no history apart from no coupling', () => {
    render({ ...CC, hidden: [], commits: 0 });
    expect(target.textContent).toContain('No commit history read');
    unmount(app!); app = null; target.remove();
    render({ ...CC, hidden: [] });
    expect(target.textContent).toContain('No hidden coupling');
  });
});
