import { describe, it, expect, vi, afterEach } from 'vitest';
import { extractCompletion, extractPreferences, mapLibraries, loadWizardData } from './loaders.js';
import { wizardState } from '$lib/wizard-state.svelte.js';
import type { LibEntry } from '$lib/types.js';

describe('extractCompletion', () => {
  it('returns pending for missing keys', () => {
    const result = extractCompletion({});
    expect(result.welcome).toBe('pending');
    expect(result.roots).toBe('pending');
    expect(result.done).toBe('pending');
  });

  it('returns done for set keys', () => {
    const result = extractCompletion({ 'setup.welcome': 'done', 'setup.roots': 'done' });
    expect(result.welcome).toBe('done');
    expect(result.roots).toBe('done');
    expect(result.scan).toBe('pending');
  });

  it('returns pending for non-done values', () => {
    const result = extractCompletion({ 'setup.welcome': 'something-else' });
    expect(result.welcome).toBe('pending');
  });

  it('covers all 11 stages', () => {
    const result = extractCompletion({});
    const keys = Object.keys(result);
    expect(keys).toContain('welcome');
    expect(keys).toContain('preferences');
    expect(keys).toContain('assistants');
    expect(keys).toContain('roots');
    expect(keys).toContain('scan');
    expect(keys).toContain('projects');
    expect(keys).toContain('libraries');
    expect(keys).toContain('instruments');
    expect(keys).toContain('inference');
    expect(keys).toContain('assignments');
    expect(keys).toContain('done');
    expect(keys).toHaveLength(11);
  });
});

describe('extractPreferences', () => {
  // #133: the glyph mode lives under its own key; the wizard must read it the
  // way Settings does, or the two would disagree about one preference.
  it('reads the glyph mode from ui.glyphs, kanji by default', () => {
    expect(extractPreferences({}).glyphs).toBe('kanji');
    expect(extractPreferences({ 'ui.glyphs': 'icon' }).glyphs).toBe('icon');
  });

  it('returns defaults when no config key', () => {
    const result = extractPreferences({});
    expect(result.displayName).toBe('');
    expect(result.contributeLearnings).toBe(true);
    expect(result.anonymizedTelemetry).toBe(false);
  });

  it('returns stored object when config key exists', () => {
    const result = extractPreferences({
      'setup.preferences': {
        displayName: 'Dev',
        contributeLearnings: false,
        anonymizedTelemetry: true,
      },
    });
    expect(result.displayName).toBe('Dev');
    expect(result.contributeLearnings).toBe(false);
    expect(result.anonymizedTelemetry).toBe(true);
  });

  it('merges partial stored object with defaults', () => {
    const result = extractPreferences({
      'setup.preferences': { displayName: 'Keiko' },
    });
    expect(result.displayName).toBe('Keiko');
    expect(result.contributeLearnings).toBe(true);
    expect(result.shareSchedule).toBe('weekly-saturday');
  });

  it('ignores non-object values', () => {
    const result = extractPreferences({ 'setup.preferences': 'not-an-object' });
    expect(result.displayName).toBe('');
  });
});

describe('mapLibraries', () => {
  const fixture: LibEntry[] = [
    { id: 'lodash', name: 'lodash', ecosystem: 'npm', version: '4.17.21', repos: ['app'], repoCount: 1 },
    { id: 'svelte', name: 'svelte', ecosystem: 'npm', version: '5.0.0', repos: ['app', 'admin'], repoCount: 2 },
  ];

  it('defaults all libs to enabled when no stored config', () => {
    const result = mapLibraries(fixture, {});
    expect(result).toHaveLength(2);
    expect(result.every(l => l.enabled)).toBe(true);
  });

  it('passes ecosystem, version, and description through to the slice', () => {
    const result = mapLibraries(fixture, {});
    const svelte = result.find(l => l.name === 'svelte')!;
    expect(svelte.ecosystem).toBe('npm');
    expect(svelte.version).toBe('5.0.0');
  });

  it('uses name as id when daemon omits it', () => {
    const result = mapLibraries(
      [{ id: '', name: 'lodash', repos: [], repoCount: 0 } as LibEntry],
      {},
    );
    expect(result[0].id).toBe('lodash');
  });

  it('honors stored wrapped/disabled lists from a JSON-string config value', () => {
    const result = mapLibraries(fixture, {
      'setup.libraries': JSON.stringify({ wrapped: ['lodash'], disabled: ['svelte'] }),
    });
    expect(result.find(l => l.name === 'lodash')?.enabled).toBe(true);
    expect(result.find(l => l.name === 'svelte')?.enabled).toBe(false);
  });

  it('honors stored wrapped/disabled when config arrives as an object', () => {
    const result = mapLibraries(fixture, {
      'setup.libraries': { wrapped: [], disabled: ['lodash'] },
    });
    expect(result.find(l => l.name === 'lodash')?.enabled).toBe(false);
    expect(result.find(l => l.name === 'svelte')?.enabled).toBe(true);
  });

  it('keeps a new lib enabled when it appears on neither stored list', () => {
    const result = mapLibraries(fixture, {
      'setup.libraries': JSON.stringify({ wrapped: ['lodash'], disabled: [] }),
    });
    // svelte is brand-new to the config — defaults to enabled.
    expect(result.find(l => l.name === 'svelte')?.enabled).toBe(true);
  });

  it('treats malformed JSON config as empty', () => {
    const result = mapLibraries(fixture, { 'setup.libraries': 'not-json{' });
    expect(result.every(l => l.enabled)).toBe(true);
  });
});

describe('loadWizardData — a config read that fails', () => {
  // The config is what says setup is complete. Read through a fallback it came
  // back `{}` on any failure, which reads as "setup incomplete": the wizard then
  // cleared the cached flag and the gate sent a set-up user back to setup.
  function stubFetch(configStatus: number) {
    vi.stubGlobal('fetch', vi.fn(async (input: string) => {
      const path = new URL(String(input), 'http://localhost').pathname;
      if (path === '/api/config') return new Response('boom', { status: configStatus });
      const body = path === '/api/libs' ? { total: 0, libs: [] }
        : path === '/api/instruments' ? { mcps: [] }
        : path === '/api/gateway/routers' ? { routers: [] }
        : [];
      return new Response(JSON.stringify(body), { status: 200 });
    }));
  }
  afterEach(() => vi.unstubAllGlobals());

  it('rejects instead of loading an empty config', async () => {
    stubFetch(500);
    await expect(loadWizardData(7744)).rejects.toThrow(/config/);
  });

  it('leaves the wizard as it was: load() reports failure, setup stays complete', async () => {
    stubFetch(500);
    wizardState.setupComplete = true;
    expect(await wizardState.load()).toBe(false);
    expect(wizardState.setupComplete).toBe(true);
  });
});
