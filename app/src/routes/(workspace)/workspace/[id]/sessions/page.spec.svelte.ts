// @vitest-environment jsdom
//
// Guards the cross-workspace leak: this is an `[id]` route, so SvelteKit reuses the
// component instance when only the param changes. The digest state used to be
// seeded once via `untrack`, which froze the fetcher's workspace id AND the initial
// rows to whichever workspace mounted first — so navigating A → B rendered A's
// sessions under B's heading.
//
// `sessions-digest.spec.svelte.ts` covers the FETCHER (`sessionsFetcher`), which
// was always correct; nothing covered the component's use of it, which is why
// this shipped.
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
  workspace: 'alpha',
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

describe('workspace sessions screen', () => {
  it('re-scopes to the new workspace when only the route param changes', () => {
    const data = $state({
      workspace: { name: 'alpha' },
      workspaceId: 'proj-alpha',
      sessions: [row({ id: 'a1', workspace: 'alpha', task: 'ALPHA_TASK' })],
      range: '7d' as const,
      error: null as string | null,
    });

    const { container, destroy } = mountComponent(Page, { data } as never);
    cleanup.push(destroy);

    expect(container.textContent).toContain('ALPHA_TASK');
    expect(container.textContent).toContain('alpha');

    // Navigate: same component instance, new param + new loaded rows.
    data.workspace = { name: 'beta' };
    data.workspaceId = 'proj-beta';
    data.sessions = [row({ id: 'b1', workspace: 'beta', task: 'BETA_TASK' })];
    flushSync();

    expect(container.textContent).toContain('BETA_TASK');
    expect(container.textContent).not.toContain('ALPHA_TASK');
  });

  it('reflects the newly loaded workspace name in the heading', () => {
    const data = $state({
      workspace: { name: 'alpha' },
      workspaceId: 'proj-alpha',
      sessions: [] as SessionRow[],
      range: '7d' as const,
      error: null as string | null,
    });
    const { container, destroy } = mountComponent(Page, { data } as never);
    cleanup.push(destroy);

    expect(container.textContent).toContain('alpha');

    data.workspace = { name: 'beta' };
    data.workspaceId = 'proj-beta';
    flushSync();

    expect(container.textContent).toContain('beta');
  });

  // #288: sessions that could not be read are an error with retry — not an
  // empty digest that reads as a quiet week. The range chips stay usable.
  it('shows the error in place of the digest when the read failed', () => {
    const data = $state({
      workspace: { name: 'alpha' },
      workspaceId: 'proj-alpha',
      sessions: null as SessionRow[] | null,
      range: '7d' as const,
      error: 'Service Unavailable' as string | null,
    });
    const { container, destroy } = mountComponent(Page, { data } as never);
    cleanup.push(destroy);

    expect(container.querySelector('[data-screen-state="error"]')?.textContent).toContain('Service Unavailable');
    expect(container.querySelector('[data-testid="session-list"], [data-testid="chart-body"], [data-testid="mini-numbers"]')).toBeNull();
    expect(container.querySelector('[data-testid="range-chips"]')).not.toBeNull();
  });

  // #293: one Sessions screen with the Observatory; a workspace's totals line
  // names no workspace count.
  it('totals this workspace without an "across N workspaces" split', () => {
    const data = $state({
      workspace: { name: 'alpha' },
      workspaceId: 'proj-alpha',
      sessions: [row({ id: 'a1' }), row({ id: 'a2' })],
      range: '7d' as const,
      error: null as string | null,
    });
    const { container, destroy } = mountComponent(Page, { data } as never);
    cleanup.push(destroy);
    const totals = container.querySelector('[data-testid="quality-tally"]')?.parentElement?.textContent ?? '';
    expect(totals).toMatch(/2\s+sessions\s+·\s+median/);
    expect(totals).not.toContain('across');
    expect(container.querySelector('[data-testid="workspace-sessions"]')).not.toBeNull();
  });
});
