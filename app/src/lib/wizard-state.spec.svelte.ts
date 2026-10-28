import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { WizardState, routerKeyStatus, saveRouterKey } from './wizard-state.svelte.js';
import { scanState } from './scan-state.svelte.js';
import { mockWizardLoadData, mockWatchRoot, mockRouter } from './setup/mock-contracts.js';

describe('WizardState', () => {
  let ws: WizardState;

  beforeEach(() => {
    ws = new WizardState();
  });

  // The settings Roots screen never fetched the roots: only the setup layout
  // calls load(), so outside setup the list started empty and stayed empty —
  // and with it the remove action (#247). `reloadRoots` is the one read.
  describe('reloadRoots', () => {
    it('replaces the cached roots with the daemon\'s', async () => {
      const fresh = [mockWatchRoot({ id: 'r9', path: '/r9' })];
      const failure = await ws.reloadRoots({ tryGetScanRoots: async () => ({ ok: true, data: fresh }) });
      expect(failure).toBeNull();
      expect(ws.roots.roots.map((r) => r.id)).toEqual(['r9']);
    });

    // A failed read is said, and the cached list survives it: an empty list
    // would read as "you have no roots".
    it('a failed read keeps the cached list and says why', async () => {
      ws.roots.roots = [mockWatchRoot({ id: 'r1' })];
      const failure = await ws.reloadRoots({
        tryGetScanRoots: async () => ({ ok: false, error: { status: 0, message: 'connection refused' } }),
      });
      expect(failure).toMatch(/could not reach the daemon/i);
      expect(ws.roots.roots.map((r) => r.id)).toEqual(['r1']);
    });
  });

  // #304: the add went through the legacy post, whose failure was `{ ok: false }`
  // with no reason — the screen kept the typed path but said nothing.
  describe('addRoot', () => {
    const added = { ok: true as const, data: { ok: true, id: 'r2', path: '/r2' } };

    it('adds the root the daemon stored and clears the input', async () => {
      ws.roots.newPath = ' /r2 ';
      const tryAddWatchRoot = vi.fn(async () => added);
      expect(await ws.addRoot({ tryAddWatchRoot })).toBeNull();
      expect(tryAddWatchRoot).toHaveBeenCalledWith('/r2');
      expect(ws.roots.roots.map((r) => [r.id, r.path, r.name])).toEqual([['r2', '/r2', 'r2']]);
      expect(ws.roots.newPath).toBe('');
    });

    it('a refused add keeps the typed path and the list, and says why', async () => {
      ws.roots.roots = [mockWatchRoot({ id: 'r1' })];
      ws.roots.newPath = '/r2';
      const failure = await ws.addRoot({
        tryAddWatchRoot: async () => ({ ok: false, error: { status: 400, message: 'not a directory' } }),
      });
      expect(failure).toMatch(/400 not a directory/);
      expect(ws.roots.newPath).toBe('/r2');
      expect(ws.roots.roots.map((r) => r.id)).toEqual(['r1']);
    });

    it('an unreachable daemon is said as such', async () => {
      ws.roots.newPath = '/r2';
      const failure = await ws.addRoot({
        tryAddWatchRoot: async () => ({ ok: false, error: { status: 0, message: 'connection refused' } }),
      });
      expect(failure).toMatch(/could not reach the daemon/i);
      expect(ws.roots.newPath).toBe('/r2');
    });

    it('an empty or already-listed path is not sent', async () => {
      ws.roots.roots = [mockWatchRoot({ id: 'r1', path: '/r1' })];
      const tryAddWatchRoot = vi.fn(async () => added);
      ws.roots.newPath = '  ';
      expect(await ws.addRoot({ tryAddWatchRoot })).toBeNull();
      ws.roots.newPath = '/r1';
      expect(await ws.addRoot({ tryAddWatchRoot })).toBeNull();
      expect(tryAddWatchRoot).not.toHaveBeenCalled();
    });
  });

  describe('hydrate', () => {
    it('populates stage status from load data', () => {
      ws.hydrate(mockWizardLoadData());
      const welcome = ws.stages.find(s => s.id === 'welcome');
      expect(welcome?.status).toBe('pending');
    });

    it('sets stage.status to done for completed stages', () => {
      ws.hydrate(mockWizardLoadData({
        completion: {
          welcome: 'done', assistants: 'pending',
          roots: 'pending', scan: 'pending', done: 'pending',
        },
      }));
      expect(ws.stages.find(s => s.id === 'welcome')?.status).toBe('done');
      expect(ws.stages.find(s => s.id === 'assistants')?.status).toBe('pending');
    });

    it('resets active flags on rehydrate', () => {
      ws.setActive('preferences');
      ws.hydrate(mockWizardLoadData());
      expect(ws.stages.every(s => !s.active)).toBe(true);
    });

    it('populates preferences slice', () => {
      ws.hydrate(mockWizardLoadData());
      expect(ws.preferences.displayName).toBe('Dev');
    });

    it('populates assistants with selected defaulting to any-variant-installed', () => {
      ws.hydrate(mockWizardLoadData());
      expect(ws.assistants.assistants).toHaveLength(2);
      expect(ws.assistants.assistants[0].selected).toBe(true);   // claude — has installed variants
      expect(ws.assistants.assistants[1].selected).toBe(false);  // cursor — no installed variants
    });

    it('seeds partStatus to idle for not-yet-configured families', () => {
      // Claude variants in the default mock are installed but not configured,
      // so every chip starts idle until the user toggles configure or a
      // prior run's status hydrates in via daemon truth.
      ws.hydrate(mockWizardLoadData());
      const claude = ws.assistants.partStatus['claude'];
      expect(claude).toBeDefined();
      expect(Object.values(claude!).every(s => s === 'idle')).toBe(true);
    });
  });

  describe('applyAssistantEvent', () => {
    it('writes part status into the slice', () => {
      ws.hydrate(mockWizardLoadData());
      ws.applyAssistantEvent({ family: 'claude', part: 'plugins', status: 'configuring' });
      expect(ws.assistants.partStatus['claude']!['plugins']).toBe('configuring');
    });

    it('stores error message on status=error transitions', () => {
      ws.hydrate(mockWizardLoadData());
      ws.applyAssistantEvent({
        family: 'claude', part: 'plugins', status: 'error',
        error: '~/.claude/agents — permission denied',
      });
      expect(ws.assistants.partStatus['claude']!['plugins']).toBe('error');
      expect(ws.assistants.partErrors['claude']!['plugins'])
        .toBe('~/.claude/agents — permission denied');
    });

    it('clears stale error on next non-error transition', () => {
      // Otherwise the consolidated error banner would persist under a
      // freshly retrying chip and the user would think the retry failed
      // again. Reducer is responsible for resetting messages when a
      // configuring/done/idle event lands.
      ws.hydrate(mockWizardLoadData());
      ws.applyAssistantEvent({ family: 'claude', part: 'plugins', status: 'error', error: 'boom' });
      ws.applyAssistantEvent({ family: 'claude', part: 'plugins', status: 'configuring' });
      expect(ws.assistants.partErrors['claude']?.['plugins']).toBeUndefined();
    });

    it('ignores events for unknown families', () => {
      // A stale SSE subscription from a prior wizard session could deliver
      // events for a family that's no longer in the loaded set; we must
      // not synthesise empty rows in partStatus on the fly.
      ws.hydrate(mockWizardLoadData());
      const before = ws.assistants.partStatus['ghost'];
      ws.applyAssistantEvent({ family: 'ghost', part: 'plugins', status: 'configuring' });
      expect(ws.assistants.partStatus['ghost']).toBe(before);
    });
  });

  // Audit item 12 (2026-10-28): the card decided its state twice with different
  // precedence (its header said "off" while a removal was configuring) and the
  // done page counted "configured" by a third rule (the daemon's variant flags).
  describe('assistantView — one view of a family, for the card and the summary', () => {
    it('in-flight work wins: a removal that cleared the switch still reads configuring', () => {
      ws.hydrate(mockWizardLoadData());
      ws.applyAssistantEvent({ family: 'claude', part: 'plugins', status: 'configuring' });
      ws.assistants.assistants[0]!.selected = false;
      const v = ws.assistantView(ws.assistants.assistants[0]!);
      expect(v).toMatchObject({ state: 'configuring', busy: true, header: { text: 'configuring…', tone: 'accent', icon: 'spinner' } });
    });

    it('is configured once every part is done, and the summary counts the same', () => {
      ws.hydrate(mockWizardLoadData());
      const claude = ws.assistants.assistants[0]!;
      expect(ws.assistantView(claude).configured).toBe(false);
      for (const p of claude.parts) ws.applyAssistantEvent({ family: 'claude', part: p.id, status: 'done' });
      expect(ws.assistantView(claude)).toMatchObject({ state: 'done', configured: true, header: { text: 'configured', tone: 'success' } });
      expect(ws.configuredAssistants).toBe(1);
    });

    it('before any live event, configured is the daemon\'s word', () => {
      const data = mockWizardLoadData();
      data.assistantFamilies[0]!.variants = data.assistantFamilies[0]!.variants.map((v) => ({ ...v, configured: true }));
      ws.hydrate(data);
      expect(ws.assistantView(ws.assistants.assistants[0]!).configured).toBe(true);
      expect(ws.configuredAssistants).toBe(1);
    });

    it('a family with nothing installed is not found, and an error is said once', () => {
      ws.hydrate(mockWizardLoadData());
      const cursor = ws.assistants.assistants[1]!;
      expect(ws.assistantView(cursor)).toMatchObject({ found: false, header: { text: 'not found' } });
      ws.applyAssistantEvent({ family: 'claude', part: 'plugins', status: 'error', error: 'denied' });
      ws.applyAssistantEvent({ family: 'claude', part: 'skills', status: 'error', error: 'denied' });
      expect(ws.assistantView(ws.assistants.assistants[0]!)).toMatchObject({ state: 'failed', error: 'denied', showError: true });
    });
  });

  describe('markFamily helpers', () => {
    it('markFamilyConfiguring sets every part to configuring', () => {
      ws.hydrate(mockWizardLoadData());
      ws.markFamilyConfiguring('claude');
      const fam = ws.assistants.partStatus['claude']!;
      expect(Object.values(fam).every(s => s === 'configuring')).toBe(true);
    });

    it('markFamilyError consolidates one message across all parts', () => {
      // The card shows a single error block; per-part chips all flip to
      // 'error' so the user sees that everything in the family is stuck,
      // not just one chip.
      ws.hydrate(mockWizardLoadData());
      ws.markFamilyError('claude', 'claude binary not found on PATH');
      const fam = ws.assistants.partStatus['claude']!;
      const errs = ws.assistants.partErrors['claude']!;
      expect(Object.values(fam).every(s => s === 'error')).toBe(true);
      expect(Object.values(errs).every(m => m === 'claude binary not found on PATH')).toBe(true);
    });

    it('markFamilyIdle resets every part to idle', () => {
      ws.hydrate(mockWizardLoadData());
      ws.markFamilyConfiguring('claude');
      ws.markFamilyIdle('claude');
      const fam = ws.assistants.partStatus['claude']!;
      expect(Object.values(fam).every(s => s === 'idle')).toBe(true);
    });

    it('populates roots slice', () => {
      ws.hydrate(mockWizardLoadData());
      expect(ws.roots.roots).toHaveLength(1);
      expect(ws.roots.roots[0].path).toBe('/Users/test/code');
    });

    it('populates scan baseline from roots', () => {
      const data = mockWizardLoadData({
        roots: [
          mockWatchRoot({ id: 'r1', repos_found: 3, scanned: true }),
          mockWatchRoot({ id: 'r2', repos_found: 2, scanned: true }),
        ],
      });
      ws.hydrate(data);
      expect(ws.scan.baseline).toEqual({
        rootCount: 2,
        repoCount: 5,
        fileCount: 0,
        scannedRootIds: ['r1', 'r2'],
      });
    });

    it('excludes unscanned roots from baseline scannedRootIds', () => {
      const data = mockWizardLoadData({
        roots: [
          mockWatchRoot({ id: 'r1', scanned: true }),
          mockWatchRoot({ id: 'r2', scanned: false }),
        ],
      });
      ws.hydrate(data);
      expect(ws.scan.baseline!.scannedRootIds).toEqual(['r1']);
      expect(ws.scan.baseline!.rootCount).toBe(1);
    });

    it('populates workspaces slice', () => {
      ws.hydrate(mockWizardLoadData());
      expect(ws.workspaces.workspaces).toHaveLength(1);
    });

    it('marks each loaded workspace as confirmed by default', () => {
      ws.hydrate(mockWizardLoadData());
      const workspace = ws.workspaces.workspaces[0];
      expect(ws.workspaces.confirmed[workspace.id]).toBe(true);
    });

    it('defaults folders to an empty array when the daemon omits them', () => {
      ws.hydrate(mockWizardLoadData({
        workspaces: [{
          id: 'p-folderless', name: 'Folderless', description: null, client: null, goal: null,
          stack: { languages: [], frameworks: [], runtimes: [], services: [] },
          icon: { kind: 'kanji', value: '空' },
          // @ts-expect-error — exercising loader robustness when folders is missing
          folders: undefined,
        }],
      }));
      expect(ws.workspaces.workspaces[0].folders).toEqual([]);
    });

    it('populates libraries slice', () => {
      ws.hydrate(mockWizardLoadData());
      expect(ws.libraries.libs).toHaveLength(1);
    });

    it('populates instruments slice', () => {
      ws.hydrate(mockWizardLoadData());
      expect(ws.instruments.mcps).toHaveLength(1);
    });
  });

  describe('firstPendingStage', () => {
    it('returns welcome when nothing done', () => {
      ws.hydrate(mockWizardLoadData());
      expect(ws.firstPendingStage).toBe('welcome');
    });

    it('returns first pending after some done', () => {
      ws.hydrate(mockWizardLoadData({
        completion: {
          welcome: 'done', assistants: 'done',
          roots: 'pending', scan: 'pending', done: 'pending',
        },
      }));
      expect(ws.firstPendingStage).toBe('roots');
    });

    it('returns done when all stages complete', () => {
      const allDone: Record<string, 'done'> = {
        welcome: 'done', assistants: 'done',
        roots: 'done', scan: 'done', done: 'done',
      };
      ws.hydrate(mockWizardLoadData({ completion: allDone }));
      expect(ws.allDone).toBe(true);
    });
  });

  describe('canAdvance', () => {
    beforeEach(() => ws.hydrate(mockWizardLoadData()));

    it('returns true for welcome (no gate)', () => {
      expect(ws.canAdvance('welcome')).toBe(true);
    });

    it('returns false for preferences when displayName empty', () => {
      ws.preferences.displayName = '';
      expect(ws.canAdvance('preferences')).toBe(false);
    });

    it('returns true for preferences when displayName set', () => {
      ws.preferences.displayName = 'Dev';
      expect(ws.canAdvance('preferences')).toBe(true);
    });

    it('returns false for preferences when displayName is whitespace only', () => {
      ws.preferences.displayName = '   ';
      expect(ws.canAdvance('preferences')).toBe(false);
    });

    it('returns false for roots when list empty', () => {
      ws.roots.roots = [];
      expect(ws.canAdvance('roots')).toBe(false);
    });

    it('returns true for roots when list has entries', () => {
      expect(ws.canAdvance('roots')).toBe(true);
    });

    it('returns false for scan when not done', () => {
      scanState.reset();
      expect(ws.canAdvance('scan')).toBe(false);
    });

    it('returns true for scan when done', () => {
      // canAdvance reads from the live scan runtime singleton — scan
      // completion can happen while the user is on another stage, so the
      // gate is driven by scanState.status rather than the wizard slice.
      scanState.status = 'done';
      expect(ws.canAdvance('scan')).toBe(true);
      scanState.reset();
    });

    it('returns true for stages with no gate', () => {
      expect(ws.canAdvance('assistants')).toBe(true);
      expect(ws.canAdvance('workspaces')).toBe(true);
      expect(ws.canAdvance('libraries')).toBe(true);
      expect(ws.canAdvance('instruments')).toBe(true);
    });
  });

  describe('preferences slice', () => {
    beforeEach(() => ws.hydrate(mockWizardLoadData()));

    it('hydrates all preference fields', () => {
      expect(ws.preferences.displayName).toBe('Dev');
      expect(ws.preferences.contributeLearnings).toBe(true);
      expect(ws.preferences.downloadCollective).toBe('weekly');
      expect(ws.preferences.correctionAggressiveness).toBe('balanced');
      expect(ws.preferences.showWelcome).toBe(true);
    });

    it('allows mutation of preference fields', () => {
      ws.preferences.displayName = 'Keiko';
      expect(ws.preferences.displayName).toBe('Keiko');
      ws.preferences.correctionAggressiveness = 'gentle';
      expect(ws.preferences.correctionAggressiveness).toBe('gentle');
    });

    it('allows mutation of toggles', () => {
      ws.preferences.contributeLearnings = false;
      expect(ws.preferences.contributeLearnings).toBe(false);
      ws.preferences.anonymizedTelemetry = true;
      expect(ws.preferences.anonymizedTelemetry).toBe(true);
    });

    it('allows mutation of dropdowns', () => {
      ws.preferences.shareSchedule = 'daily';
      expect(ws.preferences.shareSchedule).toBe('daily');
      ws.preferences.downloadCollective = 'never';
      expect(ws.preferences.downloadCollective).toBe('never');
      ws.preferences.digestCadence = 'weekly';
      expect(ws.preferences.digestCadence).toBe('weekly');
    });
  });

  describe('isStageComplete', () => {
    it('returns false when pending', () => {
      ws.hydrate(mockWizardLoadData());
      expect(ws.isStageComplete('welcome')).toBe(false);
    });

    it('returns true when done', () => {
      ws.hydrate(mockWizardLoadData({
        completion: {
          welcome: 'done', preferences: 'pending', assistants: 'pending',
          roots: 'pending', scan: 'pending', workspaces: 'pending',
          libraries: 'pending', instruments: 'pending',
          inference: 'pending', done: 'pending',
        },
      }));
      expect(ws.isStageComplete('welcome')).toBe(true);
      expect(ws.isStageComplete('preferences')).toBe(false);
    });
  });

  describe('setActive', () => {
    beforeEach(() => ws.hydrate(mockWizardLoadData()));

    it('marks the named stage active and clears others', () => {
      ws.setActive('roots');
      const roots = ws.stages.find(s => s.id === 'roots');
      expect(roots?.active).toBe(true);
      expect(ws.stages.filter(s => s.active)).toHaveLength(1);
    });

    it('moves the active flag when called again', () => {
      ws.setActive('roots');
      ws.setActive('scan');
      expect(ws.stages.find(s => s.id === 'roots')?.active).toBe(false);
      expect(ws.stages.find(s => s.id === 'scan')?.active).toBe(true);
    });

    it('clears active when given an unknown id', () => {
      ws.setActive('roots');
      ws.setActive('does-not-exist');
      expect(ws.stages.every(s => !s.active)).toBe(true);
    });
  });

  describe('stages array shape', () => {
    it('exposes brief and description as separate fields', () => {
      const roots = ws.stages.find(s => s.id === 'roots');
      expect(roots?.brief).toBe('Where your work lives.');
      expect(roots?.description).toContain('Sensei recurses');
    });

    it('initializes status as pending and active as false', () => {
      expect(ws.stages.every(s => s.status === 'pending')).toBe(true);
      expect(ws.stages.every(s => s.active === false)).toBe(true);
    });
  });

  describe('setupComplete reconciliation (single-owner contract)', () => {
    // wizardState owns the setupComplete flag and the localStorage cache
    // for it. These tests assert the writer contract: only hydrate() and
    // setCompleted() touch `sensei:setup-complete`. (appState.load() used
    // to do this — that pathway moved here in the layering correction.)
    const storage = new Map<string, string>();
    const stub = {
      getItem:    (k: string) => storage.get(k) ?? null,
      setItem:    (k: string, v: string) => storage.set(k, v),
      removeItem: (k: string) => storage.delete(k),
      clear:      () => storage.clear(),
    };

    beforeEach(() => {
      storage.clear();
      vi.stubGlobal('localStorage', stub);
    });

    it('hydrate writes "1" to localStorage when data.setupComplete=true', async () => {
      const ws = new WizardState();
      await ws.hydrate(mockWizardLoadData({ setupComplete: true }));
      expect(ws.setupComplete).toBe(true);
      expect(storage.get('sensei:setup-complete')).toBe('1');
    });

    it('hydrate clears localStorage when data.setupComplete=false', async () => {
      storage.set('sensei:setup-complete', '1');
      const ws = new WizardState();
      await ws.hydrate(mockWizardLoadData({ setupComplete: false }));
      expect(ws.setupComplete).toBe(false);
      expect(storage.has('sensei:setup-complete')).toBe(false);
    });

    it('setCompleted writes "1" and flips the flag', async () => {
      // setCompleted calls appState.setConfigs internally, which would
      // attempt to call the daemon API. Stub the appState module so this
      // test stays at the wizardState/localStorage seam.
      const appStateModule = await import('./appstate.svelte.js');
      const setConfigsSpy = vi.spyOn(appStateModule.appState, 'setConfigs')
        .mockResolvedValue(undefined);

      const ws = new WizardState();
      await ws.setCompleted();
      expect(ws.setupComplete).toBe(true);
      expect(storage.get('sensei:setup-complete')).toBe('1');
      expect(setConfigsSpy).toHaveBeenCalledWith({ setup_complete: '1' });

      setConfigsSpy.mockRestore();
    });
  });

  describe('commitStage failure paths', () => {
    // commitStage's contract is now uniform: throw on failure, return void
    // on success. The layout's catch (config/+layout.svelte) surfaces the
    // message via commitError. Previously non-`done` stages silently
    // returned false, leaving the Continue button stuck with no feedback.

    it('commitStage("done") propagates errors from setCompleted', async () => {
      const ws = new WizardState();
      vi.spyOn(ws, 'setCompleted').mockRejectedValue(new Error('boom'));
      await expect(ws.commitStage('done')).rejects.toThrow(/boom/);
      vi.restoreAllMocks();
    });

    it('commitStage propagates errors from non-done stages (no silent return)', async () => {
      const ws = new WizardState();
      // Force the libraries handler to fail by making setConfigs reject.
      // Previously this returned false silently — now it must throw so the
      // layout can show an error.
      const appStateModule = await import('./appstate.svelte.js');
      const spy = vi.spyOn(appStateModule.appState, 'setConfigs')
        .mockRejectedValue(new Error('daemon down'));

      await expect(ws.commitStage('libraries')).rejects.toThrow(/daemon down/);
      spy.mockRestore();
    });

    // The name of this test used to say "with no handler". `welcome` has a
    // handler — an EMPTY one (`async () => {}`) — and `commitStage` writes the
    // `setup.X: done` marker for any stage whose handler exists, empty or not.
    // So it always made a real daemon call, and passed only when a daemon
    // happened to be listening: green on a dev machine, `fetch failed` in the
    // pre-commit hook, which is meant to need no daemon at all.
    it('commitStage("welcome") writes its setup marker despite an empty handler', async () => {
      const ws = new WizardState();
      const appStateModule = await import('./appstate.svelte.js');
      const spy = vi.spyOn(appStateModule.appState, 'setConfigs').mockResolvedValue(undefined);

      await expect(ws.commitStage('welcome')).resolves.toBeUndefined();

      // The marker write is the reason this path touches appState at all, and
      // nothing pinned it before — the boot gate's setup gate reads these keys.
      expect(spy).toHaveBeenCalledWith({ 'setup.welcome': 'done' });
      expect(ws.stages.find(s => s.id === 'welcome')?.status).toBe('done');
      spy.mockRestore();
    });
  });

  describe('inference slice', () => {
    it('hydrates routers from load data', () => {
      const ws = new WizardState();
      ws.hydrate(mockWizardLoadData({
        routers: [
          mockRouter({ id: 'openai',   configured: true }),
          mockRouter({ id: 'ollama',   needs_key: false, configured: true }),
        ],
      }));
      expect(ws.inference.routers).toHaveLength(2);
      expect(ws.inference.routers[0].id).toBe('openai');
      expect(ws.inference.routers[0].configured).toBe(true);
      expect(ws.inference.routers[1].needsKey).toBe(false);
    });

    // #285: a key that is set but withheld by the keychain is neither
    // configured nor missing — it says why, so nobody pastes a key again.
    it('carries an unreadable key and its reason', () => {
      const ws = new WizardState();
      ws.hydrate(mockWizardLoadData({
        routers: [mockRouter({ configured: null, key_error: 'keychain access needs approval' })],
      }));
      expect(ws.inference.routers[0].configured).toBeNull();
      expect(ws.inference.routers[0].keyError).toBe('keychain access needs approval');
    });

    it('labels each key standing', () => {
      const ws = new WizardState();
      ws.hydrate(mockWizardLoadData({
        routers: [
          mockRouter({ configured: true }),
          mockRouter({ configured: false }),
          mockRouter({ needs_key: false, configured: true }),
          mockRouter({ configured: null, key_error: 'keychain access needs approval' }),
        ],
      }));
      const [set, unset, none, unreadable] = ws.inference.routers.map(routerKeyStatus);
      expect(set).toEqual({ label: 'configured ✓', tone: 'success' });
      expect(unset).toEqual({ label: 'not configured', tone: 'mute' });
      expect(none).toEqual({ label: 'no key needed', tone: 'mute' });
      expect(unreadable).toEqual({
        label: 'key unreadable: keychain access needs approval',
        tone: 'warning',
      });
    });

    // #286: a failed read is an error the Providers screen shows, not an
    // empty router list.
    it('refreshInferenceRouters rejects on a failed read and keeps the list', async () => {
      const ws = new WizardState();
      ws.hydrate(mockWizardLoadData({ routers: [mockRouter({ id: 'openai' })] }));
      vi.stubGlobal('fetch', vi.fn(async () => new Response('boom', { status: 503, statusText: 'Service Unavailable' })));
      try {
        await expect(ws.refreshInferenceRouters()).rejects.toThrow(/Service Unavailable|503/);
        expect(ws.inference.routers.map((r) => r.id)).toEqual(['openai']);
      } finally {
        vi.unstubAllGlobals();
      }
    });

    it('a load whose router read failed leaves the routers untouched', () => {
      const ws = new WizardState();
      ws.hydrate(mockWizardLoadData({ routers: [mockRouter({ id: 'openai' })] }));
      ws.hydrate(mockWizardLoadData({ routers: null, routersError: 'Service Unavailable' }));
      expect(ws.inference.routers.map((r) => r.id)).toEqual(['openai']);
    });

    it('initialises draftKey/saveState empty on hydrate', () => {
      const ws = new WizardState();
      ws.hydrate(mockWizardLoadData({ routers: [mockRouter()] }));
      const r = ws.inference.routers[0];
      expect(r.draftKey).toBe('');
      expect(r.saveState).toBe('idle');
      expect(r.saveError).toBe('');
    });
  });
});

// #288: a refresh that cannot read the daemon throws — its Settings section
// shows the reason (those catch branches were unreachable behind fallbacks) —
// and a role assignment the daemon refused is a failed commit, not a done one.
describe('WizardState — reads and writes that fail', () => {
  const failing = (status = 503) =>
    vi.fn(async () => new Response('boom', { status, statusText: 'Service Unavailable' }));
  afterEach(() => vi.unstubAllGlobals());

  for (const method of ['refreshLibraries', 'refreshInstruments', 'refreshAssignments'] as const) {
    it(`${method} throws the daemon's reason`, async () => {
      vi.stubGlobal('fetch', failing());
      const ws = new WizardState();
      ws.hydrate(mockWizardLoadData());
      await expect(ws[method]()).rejects.toThrow(/Service Unavailable/);
    });
  }

  it('the assignments commit fails when the daemon refuses a role', async () => {
    vi.stubGlobal('fetch', vi.fn(async (input: string, init?: RequestInit) => {
      const p = new URL(String(input), 'http://localhost').pathname;
      if (init?.method === 'PUT') return new Response('conflict', { status: 409, statusText: 'Conflict' });
      if (p === '/api/gateway/chains') return new Response(JSON.stringify({ chains: [] }), { status: 200 });
      return new Response('{}', { status: 200 });
    }));
    const ws = new WizardState();
    ws.hydrate(mockWizardLoadData());
    ws.assignments.pendingAssignments.inference = 'chain-x';
    await expect(ws.commitStage('assignments')).rejects.toThrow(/inference/);
  });
});

// #304: the assistant and workspace writes went through the legacy post/put.
// A refused configure came back as an empty result — no errors — so the card
// went to done; a refused workspace save only warned, and the stage was done.
describe('WizardState — assistant and workspace writes that fail (#304)', () => {
  const refuse = () => vi.fn(async () => new Response('boom', { status: 503, statusText: 'Service Unavailable' }));
  afterEach(() => vi.unstubAllGlobals());

  const claude = (ws: WizardState) => ws.assistants.assistants.find((f) => f.id === 'claude')!;

  it('a refused configure in the assistants commit fails the stage and marks the card with why', async () => {
    vi.stubGlobal('fetch', refuse());
    const ws = new WizardState();
    ws.hydrate(mockWizardLoadData());
    await expect(ws.commitStage('assistants')).rejects.toThrow(/claude/);
    expect(ws.assistants.partStatus['claude']!['plugins']).toBe('error');
    expect(ws.assistants.partErrors['claude']!['plugins']).toMatch(/Service Unavailable/);
    expect(claude(ws).variants.some((v) => v.configured)).toBe(false);
    expect(ws.stages.find((s) => s.id === 'assistants')?.status).not.toBe('done');
  });

  it('a refused toggle-on reverts the switch and says why', async () => {
    vi.stubGlobal('fetch', refuse());
    const ws = new WizardState();
    ws.hydrate(mockWizardLoadData());
    claude(ws).selected = false;
    await expect(ws.toggleAssistant('claude')).rejects.toThrow(/Service Unavailable/);
    expect(claude(ws).selected).toBe(false);
    expect(ws.assistants.partErrors['claude']!['plugins']).toMatch(/Service Unavailable/);
  });

  it('a refused toggle-off keeps the assistant configured and says why', async () => {
    vi.stubGlobal('fetch', refuse());
    const ws = new WizardState();
    ws.hydrate(mockWizardLoadData());
    for (const v of claude(ws).variants) v.configured = true;
    await expect(ws.toggleAssistant('claude')).rejects.toThrow(/Service Unavailable/);
    expect(claude(ws).selected).toBe(true);
    expect(claude(ws).variants.every((v) => v.configured)).toBe(true);
  });

  it('a refused retry is an error on the card, not done', async () => {
    vi.stubGlobal('fetch', refuse());
    const ws = new WizardState();
    ws.hydrate(mockWizardLoadData());
    await ws.retryAssistant('claude');
    expect(ws.assistants.partStatus['claude']!['plugins']).toBe('error');
    expect(ws.assistants.partErrors['claude']!['plugins']).toMatch(/Service Unavailable/);
  });

  it('a refused workspace save fails the stage, naming the workspace, and keeps the edit', async () => {
    vi.stubGlobal('fetch', refuse());
    const ws = new WizardState();
    ws.hydrate(mockWizardLoadData());
    ws.workspaces.workspaces[0].name = 'Renamed';
    await expect(ws.commitStage('workspaces')).rejects.toThrow(/Renamed.*Service Unavailable/);
    expect(ws.workspaces.workspaces[0].name).toBe('Renamed');
    expect(ws.stages.find((s) => s.id === 'workspaces')?.status).not.toBe('done');
  });

  it('a refused folder role fails the stage, naming the folder', async () => {
    vi.stubGlobal('fetch', vi.fn(async (input: string) =>
      String(input).includes('/api/folders/')
        ? new Response('boom', { status: 409, statusText: 'Conflict' })
        : new Response('{}', { status: 200 }),
    ));
    const ws = new WizardState();
    ws.hydrate(mockWizardLoadData());
    await expect(ws.commitStage('workspaces')).rejects.toThrow(/app.*Conflict/);
  });
});

// Audit item 13 (2026-10-28): Settings · Providers carried a line-for-line copy of
// the wizard's key save. One action, both callers.
describe('saveRouterKey', () => {
  const row = () => ({ id: 'openai', draftKey: '  sk-test  ', saveState: 'idle', saveError: '', configured: false, keyError: 'missing' } as never as Parameters<typeof saveRouterKey>[0]);

  it('saves the trimmed key and clears the draft', async () => {
    const r = row();
    const api = { setGatewayRouterKey: vi.fn(async () => ({ ok: true, data: { configured: true } })) };
    await saveRouterKey(r, api as never);
    expect(api.setGatewayRouterKey).toHaveBeenCalledWith('openai', 'sk-test');
    expect(r).toMatchObject({ saveState: 'done', configured: true, keyError: null, draftKey: '' });
  });

  it('says why a save failed and keeps the draft', async () => {
    const r = row();
    const api = { setGatewayRouterKey: vi.fn(async () => ({ ok: false, error: { message: 'keychain locked' } })) };
    await saveRouterKey(r, api as never);
    expect(r).toMatchObject({ saveState: 'failed', saveError: 'keychain locked', draftKey: '  sk-test  ' });
  });

  it('does nothing for an empty draft', async () => {
    const r = { ...row(), draftKey: '   ' } as Parameters<typeof saveRouterKey>[0];
    const api = { setGatewayRouterKey: vi.fn() };
    await saveRouterKey(r, api as never);
    expect(api.setGatewayRouterKey).not.toHaveBeenCalled();
  });
});

