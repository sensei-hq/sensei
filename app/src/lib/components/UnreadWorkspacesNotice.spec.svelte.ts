// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import UnreadWorkspacesNotice from './UnreadWorkspacesNotice.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

// #288: a cross-workspace screen says which workspaces it could not read.
describe('UnreadWorkspacesNotice', () => {
  it('names each workspace it could not read, with the reason', () => {
    const m = mountComponent(UnreadWorkspacesNotice, {
      unread: [{ name: 'alpha', reason: 'Bad Gateway' }, { name: 'beta', reason: 'HTTP 500' }],
    });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-unread-workspaces]');
    expect(el?.getAttribute('data-unread-workspaces')).toBe('2');
    expect(el?.textContent).toContain('Could not read 2 workspaces');
    expect(el?.textContent).toContain('alpha (Bad Gateway)');
    expect(el?.textContent).toContain('beta (HTTP 500)');
  });

  it('renders nothing when every workspace was read', () => {
    const m = mountComponent(UnreadWorkspacesNotice, { unread: [] });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-unread-workspaces]')).toBeNull();
  });

  it('says "workspace" for one', () => {
    const m = mountComponent(UnreadWorkspacesNotice, { unread: [{ name: 'alpha', reason: 'x' }] });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('Could not read 1 workspace:');
  });
});
