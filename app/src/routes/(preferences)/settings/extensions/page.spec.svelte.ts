// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; vi.unstubAllGlobals(); });

// #293: a refused toggle was a console warning and a silent re-read.
describe('Settings · Extensions — a refused toggle', () => {
  it('says why on the row and puts the box back', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 409, statusText: 'Conflict' }));
    const data = { extensions: [{ name: 'tdd', kind: 'skill', enabled: true }], error: null };
    const m = mountComponent(Page, { data } as never);
    cleanup.push(m.destroy);
    const box = m.container.querySelector('[data-testid="ext-toggle-skill-tdd"]') as HTMLInputElement;
    box.click();
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-testid="ext-error-skill-tdd"]')?.textContent).toContain('Conflict'),
    );
    expect(box.checked).toBe(true);
  });
});

const installed = [
  { name: 'tdd', kind: 'skill', enabled: true },
  { name: 'plan', kind: 'command', enabled: false },
];

const rows = (c: HTMLElement) => c.querySelectorAll('[data-testid^="ext-row-"]');
const toggleBox = (c: HTMLElement, kind: string, name: string) =>
  c.querySelector(`[data-testid="ext-toggle-${kind}-${name}"]`) as HTMLInputElement;
const okList = (list: unknown[]) => ({ ok: true, status: 200, json: async () => list });

// L5: the list is the loader's (+page.ts) read. The page renders it and makes no
// read of its own on mount.
describe('Settings · Extensions — the loader’s read', () => {
  it('draws the installed items, without a read of its own', async () => {
    const fetchSpy = vi.fn().mockResolvedValue(okList(installed));
    vi.stubGlobal('fetch', fetchSpy);
    const m = mountComponent(Page, { data: { extensions: installed, error: null } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(rows(m.container).length).toBe(2));
    expect(m.container.textContent).toContain('tdd');
    expect(m.container.textContent).toContain('plan');
    expect(fetchSpy).not.toHaveBeenCalled();
  });
});

// #288: a list that could not be read is an error with Retry — never "No
// extensions installed yet.", which is the honest empty.
describe('Settings · Extensions — a failed read', () => {
  it('shows the error with a Retry, not the empty copy', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 503, statusText: 'Service Unavailable' }));
    const m = mountComponent(Page, { data: { extensions: [], error: 'Service Unavailable' } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-screen-state="error"]')?.textContent).toContain('Service Unavailable'),
    );
    expect(m.container.querySelector('[data-action="retry"]')).not.toBeNull();
    expect(m.container.textContent).not.toContain('No extensions installed yet.');
  });
});

describe('Settings · Extensions — nothing installed', () => {
  it('shows the honest empty, not the error state', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(okList([])));
    const m = mountComponent(Page, { data: { extensions: [], error: null } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.textContent).toContain('No extensions installed yet.'));
    expect(m.container.querySelector('[data-screen-state="error"]')).toBeNull();
    expect(rows(m.container).length).toBe(0);
  });
});

// Busy and error are per row, so a slow or refused toggle never disables or
// marks its neighbours.
describe('Settings · Extensions — a refused toggle leaves its neighbour usable', () => {
  it('keeps the failure on its own row and toggles the other', async () => {
    const fetchSpy = vi.fn(async (input: unknown) => {
      const url = String(input);
      if (url.includes('/tdd/enabled')) return { ok: false, status: 409, statusText: 'Conflict' };
      return { ok: true, status: 200, json: async () => ({}) };
    });
    vi.stubGlobal('fetch', fetchSpy);
    const m = mountComponent(Page, { data: { extensions: installed, error: null } } as never);
    cleanup.push(m.destroy);

    toggleBox(m.container, 'skill', 'tdd').click();
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-testid="ext-error-skill-tdd"]')?.textContent).toContain('Conflict'),
    );
    expect(toggleBox(m.container, 'skill', 'tdd').checked).toBe(true);

    // Wait for the daemon's answer, not the click: a checkbox flips `checked`
    // on its own, and a row still saving hides any error behind "saving…", so
    // asserting there would pass even if the error leaked to every row.
    const planRow = () => m.container.querySelector('[data-testid="ext-row-command-plan"]')!;
    toggleBox(m.container, 'command', 'plan').click();
    await vi.waitFor(() => expect(planRow().querySelector('.extension-enabled')?.textContent).toBe('on'));
    expect(planRow().textContent).not.toContain('saving…');
    expect(toggleBox(m.container, 'command', 'plan').checked).toBe(true);
    expect(m.container.querySelector('[data-testid="ext-error-command-plan"]')).toBeNull();
    expect(m.container.querySelector('[data-testid="ext-error-skill-tdd"]')?.textContent).toContain('Conflict');
  });
});

describe('Settings · Extensions — a slow toggle', () => {
  it('marks only the row being saved', async () => {
    let answer: (v: unknown) => void = () => {};
    const pending = new Promise((resolve) => {
      answer = resolve;
    });
    vi.stubGlobal('fetch', vi.fn().mockReturnValue(pending));
    const m = mountComponent(Page, { data: { extensions: installed, error: null } } as never);
    cleanup.push(m.destroy);
    const row = (kind: string, name: string) =>
      m.container.querySelector(`[data-testid="ext-row-${kind}-${name}"]`)!;

    toggleBox(m.container, 'skill', 'tdd').click();
    await vi.waitFor(() => expect(row('skill', 'tdd').textContent).toContain('saving…'));
    expect(toggleBox(m.container, 'skill', 'tdd').disabled).toBe(true);
    expect(toggleBox(m.container, 'command', 'plan').disabled).toBe(false);
    expect(row('command', 'plan').textContent).not.toContain('saving…');

    answer({ ok: true, status: 200, json: async () => ({}) });
    await vi.waitFor(() => expect(row('skill', 'tdd').textContent).not.toContain('saving…'));
  });
});

// Phase B: Retry re-reads this page's list through its read, not
// invalidateAll() — which re-runs every load on the settings route, the
// libraries/knowledge-sources ruling.
describe('Settings · Extensions — Retry after a failed read', () => {
  it.skip('pending: L5 settings/extensions — the Retry re-reads the list through its read', async () => {
    const fetchSpy = vi.fn().mockResolvedValue(okList(installed));
    vi.stubGlobal('fetch', fetchSpy);
    const m = mountComponent(Page, { data: { extensions: [], error: 'Service Unavailable' } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.querySelector('[data-action="retry"]')).not.toBeNull());

    (m.container.querySelector('[data-action="retry"]') as HTMLButtonElement).click();

    await vi.waitFor(() => expect(rows(m.container).length).toBe(2));
    expect(m.container.querySelector('[data-screen-state="error"]')).toBeNull();
    const urls = fetchSpy.mock.calls.map((c) => String(c[0]));
    expect(urls).toHaveLength(1);
    expect(urls[0]).toContain('/api/install/installed');
  });
});

// Phase B: a re-read that fails again is the new reason, not the empty one —
// the same #288 hazard as the first read.
describe('Settings · Extensions — Retry that fails again', () => {
  it.skip('pending: L5 settings/extensions — a failed re-read stays the error, never the empty', async () => {
    const fetchSpy = vi.fn().mockResolvedValue({ ok: false, status: 502, statusText: 'Bad Gateway' });
    vi.stubGlobal('fetch', fetchSpy);
    const m = mountComponent(Page, { data: { extensions: [], error: 'Service Unavailable' } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.querySelector('[data-action="retry"]')).not.toBeNull());

    (m.container.querySelector('[data-action="retry"]') as HTMLButtonElement).click();

    await vi.waitFor(() => expect(fetchSpy).toHaveBeenCalledTimes(1));
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-screen-state="error"]')?.textContent).toContain('Bad Gateway'),
    );
    expect(m.container.querySelector('[data-action="retry"]')).not.toBeNull();
    expect(m.container.textContent).not.toContain('No extensions installed yet.');
  });
});

// L0 ruling on tests r1 (L1): `enabled ?? true` in readExtensions invents a
// state the daemon did not send (D83). A row that did not say is unknown, and
// the screen says so instead of drawing it as on.
describe('Settings · Extensions — an item that did not say whether it is on', () => {
  it.skip('pending: L5 settings/extensions — an item with no `enabled` reads as unknown, never on', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(okList([])));
    const m = mountComponent(Page, {
      data: { extensions: [{ name: 'tdd', kind: 'skill', enabled: null }], error: null },
    } as never);
    cleanup.push(m.destroy);
    const label = () =>
      m.container.querySelector('[data-testid="ext-row-skill-tdd"] .extension-enabled')!;
    await vi.waitFor(() => expect(label().textContent).toBe('unknown'));
    expect(toggleBox(m.container, 'skill', 'tdd').checked).toBe(false);
  });
});
