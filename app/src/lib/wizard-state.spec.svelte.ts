import { describe, it, expect, beforeEach, vi } from 'vitest';
import { WizardState } from './wizard-state.svelte.js';
import { scanState } from './scan-state.svelte.js';
import { mockWizardLoadData, mockWatchRoot, mockRouter } from './setup/mock-contracts.js';

describe('WizardState', () => {
  let ws: WizardState;

  beforeEach(() => {
    ws = new WizardState();
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
          welcome: 'done', preferences: 'pending', assistants: 'pending',
          roots: 'pending', scan: 'pending', projects: 'pending',
          libraries: 'pending', instruments: 'pending',
          inference: 'pending', done: 'pending',
        },
      }));
      expect(ws.stages.find(s => s.id === 'welcome')?.status).toBe('done');
      expect(ws.stages.find(s => s.id === 'preferences')?.status).toBe('pending');
    });

    it('resets active flags on rehydrate', () => {
      ws.setActive('preferences');
      ws.hydrate(mockWizardLoadData());
      expect(ws.stages.every(s => !s.active)).toBe(true);
    });

    it('populates preferences slice', () => {
      ws.hydrate(mockWizardLoadData());
      expect(ws.preferences.displayName).toBe('Jerry');
    });

    it('populates assistants with selected defaulting to any-variant-installed', () => {
      ws.hydrate(mockWizardLoadData());
      expect(ws.assistants.assistants).toHaveLength(2);
      expect(ws.assistants.assistants[0].selected).toBe(true);   // claude — has installed variants
      expect(ws.assistants.assistants[1].selected).toBe(false);  // cursor — no installed variants
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

    it('populates projects slice', () => {
      ws.hydrate(mockWizardLoadData());
      expect(ws.projects.projects).toHaveLength(1);
    });

    it('marks each loaded project as confirmed by default', () => {
      ws.hydrate(mockWizardLoadData());
      const project = ws.projects.projects[0];
      expect(ws.projects.confirmed[project.id]).toBe(true);
    });

    it('defaults folders to an empty array when the daemon omits them', () => {
      ws.hydrate(mockWizardLoadData({
        projects: [{
          id: 'p-folderless', name: 'Folderless', description: null, client: null, goal: null,
          stack: { languages: [], frameworks: [], runtimes: [], services: [] },
          icon: { kind: 'kanji', value: '空' },
          // @ts-expect-error — exercising loader robustness when folders is missing
          folders: undefined,
        }],
      }));
      expect(ws.projects.projects[0].folders).toEqual([]);
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
          welcome: 'done', preferences: 'done', assistants: 'done',
          roots: 'pending', scan: 'pending', projects: 'pending',
          libraries: 'pending', instruments: 'pending',
          inference: 'pending', done: 'pending',
        },
      }));
      expect(ws.firstPendingStage).toBe('roots');
    });

    it('returns done when all stages complete', () => {
      const allDone: Record<string, 'done'> = {
        welcome: 'done', preferences: 'done', assistants: 'done',
        roots: 'done', scan: 'done', projects: 'done',
        libraries: 'done', instruments: 'done',
        inference: 'done', done: 'done',
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
      ws.preferences.displayName = 'Jerry';
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
      expect(ws.canAdvance('projects')).toBe(true);
      expect(ws.canAdvance('libraries')).toBe(true);
      expect(ws.canAdvance('instruments')).toBe(true);
    });
  });

  describe('preferences slice', () => {
    beforeEach(() => ws.hydrate(mockWizardLoadData()));

    it('hydrates all preference fields', () => {
      expect(ws.preferences.displayName).toBe('Jerry');
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
          roots: 'pending', scan: 'pending', projects: 'pending',
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

    it('commitStage("welcome") with no handler resolves to void and marks stage done', async () => {
      const ws = new WizardState();
      await expect(ws.commitStage('welcome')).resolves.toBeUndefined();
      expect(ws.stages.find(s => s.id === 'welcome')?.status).toBe('done');
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
