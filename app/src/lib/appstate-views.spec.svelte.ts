/**
 * The view panel store (D87, screen-shell contract §4): every view's remembered
 * panels live in ONE config entry, `ui.views`, loaded once with the config and
 * re-synced across windows by `onConfigSaved`. A view with no entry is a first
 * visit: everything open.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import * as appstate from './appstate.svelte.js';
import type { PanelState } from '@rokkit/app';

// Phase A (pending: shell-store): the store's API does not exist yet, so it is
// reached through this shape; phase B imports it directly and drops the cast.
interface ViewsApi {
  viewPanels(viewId: string): PanelState;
  setViewPanels(viewId: string, next: PanelState): Promise<void>;
  migrateLegacyViews(viewIds: string[]): Promise<void>;
  readonly viewsError: string | null;
  onConfigSaved: ((keys: string[]) => void) | null;
  load(): Promise<void>;
}
const AppState = appstate.AppState as unknown as new () => ViewsApi;
type AppState = ViewsApi;
const VIEWS_KEY = (appstate as unknown as { VIEWS_KEY: string }).VIEWS_KEY;
import { STORAGE_KEYS } from './storage-keys.js';

const apiMock = {
  tryGetConfig: vi.fn(),
  trySetConfig: vi.fn(),
  tryDeleteConfig: vi.fn(),
  tryListWorkspaces: vi.fn(),
};

vi.mock('./api.js', async (importOriginal) => ({
  failureReason: (await importOriginal<typeof import('./api.js')>()).failureReason,
  senseiApi: () => apiMock,
}));

const storage = new Map<string, string>();
vi.stubGlobal('localStorage', {
  getItem: (key: string) => storage.get(key) ?? null,
  setItem: (key: string, value: string) => storage.set(key, value),
  removeItem: (key: string) => storage.delete(key),
  clear: () => storage.clear(),
});

const win = (globalThis as unknown as { window?: Record<string, unknown> }).window
  ?? ((globalThis as unknown as { window: Record<string, unknown> }).window = {});
win.__TAURI__ = {};

const STRUCTURE = 'workspace.diagrams.structure';
const ER = 'workspace.diagrams.er';
const LEGACY = [
  STORAGE_KEYS.diagramsAbout,
  STORAGE_KEYS.diagramsStats,
  STORAGE_KEYS.diagramsControls,
  STORAGE_KEYS.diagramsSide,
];

/** The `ui.views` map the last successful write sent the daemon. */
function lastWrittenViews(): Record<string, unknown> {
  const call = apiMock.trySetConfig.mock.calls.at(-1);
  expect(call, 'a write reached the daemon').toBeTruthy();
  return JSON.parse((call![0] as Record<string, string>)[VIEWS_KEY]);
}

async function loaded(config: Record<string, string>): Promise<AppState> {
  apiMock.tryGetConfig.mockResolvedValueOnce({ ok: true, data: config });
  const state = new AppState();
  await state.load();
  return state;
}

describe.skip('pending: shell-store — AppState · view panels (ui.views)', () => {
  beforeEach(() => {
    storage.clear();
    vi.clearAllMocks();
    apiMock.trySetConfig.mockResolvedValue({ ok: true, data: undefined });
  });

  it('the key is ui.views, beside ui.glyphs', () => {
    expect(VIEWS_KEY).toBe('ui.views');
  });

  it('a view with no entry is a first visit: an empty state, every panel open', async () => {
    const state = await loaded({ [VIEWS_KEY]: JSON.stringify({ [ER]: { info: false } }) });
    expect(state.viewPanels(STRUCTURE)).toEqual({});
    expect(state.viewsError).toBeNull();
  });

  it('reads each view by its own id: two views keep two states', async () => {
    const state = await loaded({
      [VIEWS_KEY]: JSON.stringify({ [STRUCTURE]: { info: false }, [ER]: { side: false, stats: true } }),
    });
    expect(state.viewPanels(STRUCTURE)).toEqual({ info: false });
    expect(state.viewPanels(ER)).toEqual({ side: false, stats: true });
  });

  it('a save writes the whole map under ui.views, keeps the other views, and tells the other windows', async () => {
    const state = await loaded({ [VIEWS_KEY]: JSON.stringify({ [ER]: { side: false } }) });
    const told: string[][] = [];
    state.onConfigSaved = (keys) => told.push(keys);
    await state.setViewPanels(STRUCTURE, { info: false, stats: true });
    expect(lastWrittenViews()).toEqual({ [ER]: { side: false }, [STRUCTURE]: { info: false, stats: true } });
    expect(state.viewPanels(STRUCTURE)).toEqual({ info: false, stats: true });
    expect(state.viewPanels(ER)).toEqual({ side: false });
    expect(told).toEqual([[VIEWS_KEY]]);
  });

  it('a toggle replaces that view\'s whole state, not a merge into the old one', async () => {
    const state = await loaded({ [VIEWS_KEY]: JSON.stringify({ [STRUCTURE]: { info: false, side: false } }) });
    await state.setViewPanels(STRUCTURE, { info: true });
    expect(state.viewPanels(STRUCTURE)).toEqual({ info: true });
  });

  it('two saves in a row both land: the second builds on the first', async () => {
    const state = await loaded({});
    await state.setViewPanels(STRUCTURE, { info: false });
    await state.setViewPanels(ER, { side: false });
    expect(lastWrittenViews()).toEqual({ [STRUCTURE]: { info: false }, [ER]: { side: false } });
  });

  it('a save the daemon refused throws, keeps the stored state, and says why', async () => {
    const state = await loaded({ [VIEWS_KEY]: JSON.stringify({ [STRUCTURE]: { info: false } }) });
    apiMock.trySetConfig.mockResolvedValueOnce({ ok: false, error: { status: 500, message: 'boom' } });
    await expect(state.setViewPanels(STRUCTURE, { info: true })).rejects.toThrow(/boom/);
    expect(state.viewPanels(STRUCTURE)).toEqual({ info: false });
    expect(state.viewsError).toMatch(/boom/);
    // The next save that lands clears it.
    await state.setViewPanels(STRUCTURE, { info: true });
    expect(state.viewsError).toBeNull();
  });

  // Without a good config read the cache is not the daemon's map: writing the
  // whole map from it would erase every other view's remembered panels.
  it('refuses to save before a good config read: the daemon\'s map is never overwritten from a stale cache', async () => {
    apiMock.tryGetConfig.mockResolvedValueOnce({ ok: false, error: { status: 0, message: 'Load failed' } });
    const state = new AppState();
    vi.useFakeTimers();
    try {
      await state.load();
      await expect(state.setViewPanels(STRUCTURE, { info: false })).rejects.toThrow();
      expect(apiMock.trySetConfig).not.toHaveBeenCalled();
      expect(state.viewPanels(STRUCTURE)).toEqual({});
    } finally {
      vi.useRealTimers();
    }
  });

  it('refuses to save before any read at all', async () => {
    const state = new AppState();
    await expect(state.setViewPanels(STRUCTURE, { info: false })).rejects.toThrow();
    expect(apiMock.trySetConfig).not.toHaveBeenCalled();
  });

  it('an unreadable ui.views is said, never read as an empty map', async () => {
    const state = await loaded({ [VIEWS_KEY]: '{not json' });
    expect(state.viewsError).toMatch(/ui\.views/);
    // The view opens as a first visit — the contract's rule — but the error is there to show.
    expect(state.viewPanels(STRUCTURE)).toEqual({});
  });

  it('a stored entry that is not a panel state is said, and only the valid keys are read', async () => {
    const state = await loaded({
      [VIEWS_KEY]: JSON.stringify({ [STRUCTURE]: { info: false, bogus: false, side: 'no' }, [ER]: 3 }),
    });
    expect(state.viewPanels(STRUCTURE)).toEqual({ info: false });
    expect(state.viewPanels(ER)).toEqual({});
    expect(state.viewsError).toMatch(/ui\.views/);
  });

  it('another window\'s save reaches this one through the config re-read', async () => {
    const state = await loaded({});
    expect(state.viewPanels(ER)).toEqual({});
    apiMock.tryGetConfig.mockResolvedValueOnce({
      ok: true, data: { [VIEWS_KEY]: JSON.stringify({ [ER]: { stats: false } }) },
    });
    await state.load();
    expect(state.viewPanels(ER)).toEqual({ stats: false });
  });

  // ── the one-time migration of the diagrams' localStorage toggles ──────────

  describe('migrateLegacyViews', () => {
    const IDS = [STRUCTURE, ER];

    it('maps about → info, stats → stats, controls → filters, side → side, into every diagram view, then deletes the keys', async () => {
      storage.set(STORAGE_KEYS.diagramsAbout, 'closed');
      storage.set(STORAGE_KEYS.diagramsStats, 'open');
      storage.set(STORAGE_KEYS.diagramsControls, 'closed');
      storage.set(STORAGE_KEYS.diagramsSide, 'closed');
      const state = await loaded({});
      await state.migrateLegacyViews(IDS);
      const want = { info: false, stats: true, filters: false, side: false };
      expect(lastWrittenViews()).toEqual({ [STRUCTURE]: want, [ER]: want });
      expect(state.viewPanels(ER)).toEqual(want);
      for (const key of LEGACY) expect(storage.has(key), key).toBe(false);
    });

    it('a key that was never set stays absent (open), not stored as open or closed', async () => {
      storage.set(STORAGE_KEYS.diagramsSide, 'closed');
      const state = await loaded({});
      await state.migrateLegacyViews(IDS);
      expect(state.viewPanels(STRUCTURE)).toEqual({ side: false });
    });

    it('never overwrites a view that already has an entry', async () => {
      storage.set(STORAGE_KEYS.diagramsAbout, 'closed');
      const state = await loaded({ [VIEWS_KEY]: JSON.stringify({ [ER]: { info: true, side: false } }) });
      await state.migrateLegacyViews(IDS);
      expect(state.viewPanels(ER)).toEqual({ info: true, side: false });
      expect(state.viewPanels(STRUCTURE)).toEqual({ info: false });
    });

    it('with no legacy keys there is nothing to migrate and nothing is written', async () => {
      const state = await loaded({});
      await state.migrateLegacyViews(IDS);
      expect(apiMock.trySetConfig).not.toHaveBeenCalled();
    });

    it('runs once: a second call writes nothing', async () => {
      storage.set(STORAGE_KEYS.diagramsAbout, 'closed');
      const state = await loaded({});
      await state.migrateLegacyViews(IDS);
      await state.migrateLegacyViews(IDS);
      expect(apiMock.trySetConfig).toHaveBeenCalledTimes(1);
    });

    it('a failed write keeps the legacy keys, so the next start migrates again, and throws', async () => {
      storage.set(STORAGE_KEYS.diagramsAbout, 'closed');
      const state = await loaded({});
      apiMock.trySetConfig.mockResolvedValueOnce({ ok: false, error: { status: 500, message: 'boom' } });
      await expect(state.migrateLegacyViews(IDS)).rejects.toThrow(/boom/);
      expect(storage.get(STORAGE_KEYS.diagramsAbout)).toBe('closed');
      expect(state.viewPanels(STRUCTURE)).toEqual({});
    });

    it('does not run before a good config read, and keeps the keys', async () => {
      storage.set(STORAGE_KEYS.diagramsAbout, 'closed');
      const state = new AppState();
      await expect(state.migrateLegacyViews(IDS)).rejects.toThrow();
      expect(apiMock.trySetConfig).not.toHaveBeenCalled();
      expect(storage.get(STORAGE_KEYS.diagramsAbout)).toBe('closed');
    });
  });
});
