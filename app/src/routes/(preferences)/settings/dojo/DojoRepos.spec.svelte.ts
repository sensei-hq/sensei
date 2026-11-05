// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { flushSync } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import Harness from './DojoRepos.harness.svelte';
import type { DojoGroup, RepoView } from './dojo-connection.js';

// WP8 (D46; L0 ruling: the app toggle is L2's): the switch shows only on a row its
// dōjō lets this person change, and a refused change shows its refusal Notice.

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((f) => f());
  cleanup = [];
});

const row = (key: string, toggle: RepoView['toggle']): RepoView => ({
  key, name: key.split('/').slice(1).join('/'), state: 'synced', status: 'Syncing', category: 'syncing', notice: null, toggle,
});
const group = (repos: RepoView[]): DojoGroup => ({
  id: 'p', name: 'jerrythomas', kind: 'personal', kanji: '己', isOrg: false, repos, empty: false, count: '1 of 1 syncing',
});

import type { ScreenNoticeProps } from './dojo-connection-state.svelte.js';

type Props = { dojos: DojoGroup[]; ontoggle?: (key: string, elected: boolean) => void; electionNotice?: ScreenNoticeProps | null };

function mount(props: Props) {
  const m = mountComponent(Harness, props);
  cleanup.push(m.destroy);
  return m.container;
}

describe('DojoRepos · the sync switch', () => {
  it.skip('pending: wp8-daemon — shows on a row the person may change, and is absent on one they may not', () => {
    const root = mount({
      dojos: [group([row('github.com/me/notes', { on: true }), row('github.com/acme/web', null)])],
    });
    const notes = root.querySelector('[data-repository="github.com/me/notes"] [data-repo-toggle]');
    expect(notes?.getAttribute('aria-pressed')).toBe('true');
    expect(root.querySelector('[data-repository="github.com/acme/web"] [data-repo-toggle]')).toBeNull();
  });

  it.skip('pending: wp8-daemon — switching hands the opposite choice back', () => {
    const ontoggle = vi.fn();
    const root = mount({ dojos: [group([row('github.com/me/notes', { on: true })])], ontoggle });
    (root.querySelector('[data-repo-toggle]') as HTMLElement).click();
    flushSync();
    expect(ontoggle).toHaveBeenCalledWith('github.com/me/notes', false);
  });

  it.skip('pending: wp8-daemon — a refused change shows its refusal Notice', () => {
    const root = mount({
      dojos: [group([row('github.com/acme/web', null)])],
      electionNotice: {
        tone: 'refusal',
        title: 'Only this organisation’s admin can change this repository’s sync',
        detail: 'only an admin may change this',
        code: 'not_yours_to_change',
      },
    });
    const notice = root.querySelector('[data-election-notice]');
    expect(notice?.textContent).toContain('Only this organisation’s admin can change this repository’s sync');
    expect(notice?.textContent).toContain('only an admin may change this');
  });
});
