// @vitest-environment jsdom
//
// Guards the cross-project leak: this is an `[id]` route, so SvelteKit reuses the
// component instance when only the param changes. The digest state used to be
// seeded once via `untrack`, which froze the fetcher's project id AND the initial
// rows to whichever project mounted first — so navigating A → B rendered A's
// sessions under B's heading.
//
// The existing `project-sessions.spec.ts` covers the FETCHER, which was always
// correct; nothing covered the component's use of it, which is why this shipped.
// Per test-mount's contract, navigation is simulated by mutating a reactive
// `data` prop rather than remounting — a remount would pass even with the bug.
import { describe, it, expect, afterEach } from 'vitest';
import { flushSync } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';
import type { SessionRow } from '$lib/types.js';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

const row = (over: Partial<SessionRow> = {}): SessionRow => ({
  id: 's1',
  project: 'alpha',
  task: 'ALPHA_TASK',
  summary: null,
  outcome: 'completed',
  ftr: true,
  turns: 10,
  corrections: 0,
  startedAt: new Date().toISOString(),
  completedAt: null,
  agent: 'claude',
  ...over,
});

describe('project sessions screen', () => {
  it('re-scopes to the new project when only the route param changes', () => {
    const data = $state({
      project: { name: 'alpha' },
      projectId: 'proj-alpha',
      sessions: [row({ id: 'a1', project: 'alpha', task: 'ALPHA_TASK' })],
      range: '7d' as const,
    });

    const { container, destroy } = mountComponent(Page, { data } as never);
    cleanup.push(destroy);

    expect(container.textContent).toContain('ALPHA_TASK');
    expect(container.textContent).toContain('alpha');

    // Navigate: same component instance, new param + new loaded rows.
    data.project = { name: 'beta' };
    data.projectId = 'proj-beta';
    data.sessions = [row({ id: 'b1', project: 'beta', task: 'BETA_TASK' })];
    flushSync();

    expect(container.textContent).toContain('BETA_TASK');
    expect(container.textContent).not.toContain('ALPHA_TASK');
  });

  it('reflects the newly loaded project name in the heading', () => {
    const data = $state({
      project: { name: 'alpha' },
      projectId: 'proj-alpha',
      sessions: [] as SessionRow[],
      range: '7d' as const,
    });
    const { container, destroy } = mountComponent(Page, { data } as never);
    cleanup.push(destroy);

    expect(container.textContent).toContain('alpha');

    data.project = { name: 'beta' };
    data.projectId = 'proj-beta';
    flushSync();

    expect(container.textContent).toContain('beta');
  });
});
