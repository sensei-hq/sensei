// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';
import type { PageData } from './$types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const empty = { skill: [], agent: [], rule: [], lint: [], other: [] };
const mount = (data: object) => {
  const m = mountComponent(Page, { data: data as unknown as PageData });
  cleanup.push(m.destroy);
  return m;
};

// #288: each lane shows its own failure — never the empty state.
describe('Upgrades /+page — a failed read', () => {
  it('a dōjō that could not be read says so, in its lane', () => {
    const m = mount({ dojoUpgrades: [], dojoUnread: 0, dojoError: 'Bad Gateway', buckets: empty, total: 0, unread: [], localError: null });
    const lane = m.container.querySelector('[data-lane="dojo"]');
    expect(lane?.querySelector('[data-screen-state="error"]')?.textContent).toContain('Bad Gateway');
    expect(lane?.textContent).not.toContain('no dōjō upgrades yet');
  });

  it('a local lane that could not be read says so, in its lane', () => {
    const m = mount({ dojoUpgrades: [], dojoUnread: 0, dojoError: null, buckets: null, total: 0, unread: [], localError: 'Service Unavailable' });
    const lane = m.container.querySelector('[data-lane="local"]');
    expect(lane?.querySelector('[data-screen-state="error"]')?.textContent).toContain('Service Unavailable');
    expect(lane?.textContent).not.toContain('no upgrades waiting');
  });

  it('names the workspaces it could not read', () => {
    const m = mount({ dojoUpgrades: [], dojoUnread: 0, dojoError: null, buckets: empty, total: 0, unread: [{ name: 'beta', reason: 'x' }], localError: null });
    expect(m.container.querySelector('[data-lane="local"] [data-unread-workspaces]')?.textContent).toContain('beta (x)');
  });
});
