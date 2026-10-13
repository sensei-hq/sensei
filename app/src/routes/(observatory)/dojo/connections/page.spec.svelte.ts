// @vitest-environment jsdom
// Signing in is joining (#236 step 4e): the dōjō takes the person's session
// only, so there is no device token to pair with. The screen lists the
// memberships the signed-in persona's sync brought in and sends you to sign in.
import { describe, it, expect, afterEach, vi } from 'vitest';
import { tick } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';
import type { DojoMembership } from '$lib/types.js';

vi.mock('$app/navigation', () => ({ goto: vi.fn(), invalidateAll: vi.fn() }));

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

async function mount(connections: DojoMembership[], error: string | null = null) {
  const m = mountComponent(Page, { data: { connections, error } });
  cleanup.push(m.destroy);
  await tick();
  return m.container;
}

describe('/dojo/connections', () => {
  it('offers no pairing form and asks for no device token', async () => {
    const page = await mount([]);
    expect(page.querySelector('[data-connect-toggle]')).toBeNull();
    expect(page.querySelector('[data-connect-form]')).toBeNull();
    expect(page.textContent?.toLowerCase()).not.toContain('device token');
  });

  it('sends you to sign in, where every dōjō you belong to joins', async () => {
    const page = await mount([]);
    const link = page.querySelector('a[data-sign-in]') as HTMLAnchorElement | null;
    expect(link?.getAttribute('href')).toBe('/settings/dojo');
    expect(page.textContent).toMatch(/sign in/i);
  });

  // A daemon that cannot list memberships is an error to retry — never "no
  // dōjō yet, sign in", which would send you to fix the wrong thing.
  it('shows a failed read as an error, not as an empty list', async () => {
    const page = await mount([], 'daemon unreachable');
    expect(page.textContent).toContain('daemon unreachable');
    expect(page.textContent).not.toMatch(/no dōjō yet/i);
  });
});
