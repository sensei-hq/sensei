// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import UnreadProjectsNotice from './UnreadProjectsNotice.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

// #288: a cross-project screen says which projects it could not read.
describe('UnreadProjectsNotice', () => {
  it('names each project it could not read, with the reason', () => {
    const m = mountComponent(UnreadProjectsNotice, {
      unread: [{ name: 'alpha', reason: 'Bad Gateway' }, { name: 'beta', reason: 'HTTP 500' }],
    });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-unread-projects]');
    expect(el?.getAttribute('data-unread-projects')).toBe('2');
    expect(el?.textContent).toContain('Could not read 2 projects');
    expect(el?.textContent).toContain('alpha (Bad Gateway)');
    expect(el?.textContent).toContain('beta (HTTP 500)');
  });

  it('renders nothing when every project was read', () => {
    const m = mountComponent(UnreadProjectsNotice, { unread: [] });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-unread-projects]')).toBeNull();
  });

  it('says "project" for one', () => {
    const m = mountComponent(UnreadProjectsNotice, { unread: [{ name: 'alpha', reason: 'x' }] });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('Could not read 1 project:');
  });
});
