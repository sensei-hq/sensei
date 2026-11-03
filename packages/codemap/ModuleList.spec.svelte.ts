// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import ModuleList from './ModuleList.svelte';
import type { ListView } from './list';

let app: Record<string, unknown> | null = null;
let target: HTMLElement;
afterEach(() => { if (app) unmount(app); app = null; target?.remove(); });

const VIEW: ListView = {
  groups: [{
    label: 'L0 · 2',
    rows: [
      { id: 'p/a', label: 'p/a', sub: '50 symbols', roles: { code: 3, test: 1, docs: 0, config: 0, generated: 0 }, badges: [{ text: 'upward', tone: 'danger' }], open: true },
      { id: 'L0 → L1', label: 'L0 → L1', sub: '4 calls + imports', roles: null, badges: [], open: false },
    ],
  }],
};

describe('ModuleList', () => {
  it('draws each row 44px or taller, with its role bar and badges; only a module opens', () => {
    const onopen = vi.fn();
    target = document.body.appendChild(document.createElement('div'));
    app = mount(ModuleList, { target, props: { view: VIEW, onopen } });
    flushSync();
    const rows = [...target.querySelectorAll('[data-testid^="list-row-"]')] as HTMLElement[];
    expect(rows.map((r) => r.className.includes('min-h-11'))).toEqual([true, true]);
    expect(target.querySelector('[data-testid="list-row-p/a"] [data-testid="role-bar"]')?.children.length).toBe(2);
    expect(target.querySelector('[data-testid="list-row-p/a"]')?.textContent).toContain('upward');
    (target.querySelector('[data-testid="list-row-p/a"]') as HTMLButtonElement).click();
    (target.querySelector('[data-testid="list-row-L0 → L1"]') as HTMLElement).click();
    expect(onopen.mock.calls).toEqual([['p/a']]);
  });
});
