import { describe, it, expect } from 'vitest';
import { DEFAULT_PREFERENCES, fromPreferencesForm, toPreferencesForm } from './preferences-form.js';

describe('toPreferencesForm', () => {
  it('yields defaults when the daemon has no prefs stored', () => {
    expect(toPreferencesForm({})).toEqual(DEFAULT_PREFERENCES);
  });

  it('falls back to defaults on corrupt JSON so the page never fails to render', () => {
    expect(toPreferencesForm({ 'setup.preferences': '{not json' })).toEqual(DEFAULT_PREFERENCES);
  });

  it('merges partial JSON on top of the defaults (a missing key stays default)', () => {
    const merged = toPreferencesForm({
      'setup.preferences': JSON.stringify({ digestCadence: 'weekly' }),
    });
    expect(merged.digestCadence).toBe('weekly');
    expect(merged.correctionAggressiveness).toBe(DEFAULT_PREFERENCES.correctionAggressiveness);
  });

  it('prefers user_name over displayName in the JSON blob', () => {
    const merged = toPreferencesForm({
      'setup.preferences': JSON.stringify({ displayName: 'stale' }),
      'user_name': 'fresh',
    });
    expect(merged.displayName).toBe('fresh');
  });

  it('leaves the JSON-blob displayName intact when user_name is absent', () => {
    const merged = toPreferencesForm({
      'setup.preferences': JSON.stringify({ displayName: 'jerry' }),
    });
    expect(merged.displayName).toBe('jerry');
  });
});

describe('fromPreferencesForm', () => {
  it('writes both keys so downstream readers (greeter + wizard) stay consistent', () => {
    const out = fromPreferencesForm({
      ...DEFAULT_PREFERENCES,
      displayName: 'jerry',
      digestCadence: 'weekly',
    });
    expect(out['user_name']).toBe('jerry');
    const round = JSON.parse(out['setup.preferences']);
    expect(round.digestCadence).toBe('weekly');
    expect(round.displayName).toBe('jerry');
  });

  it('round-trips: fromForm ∘ toForm is identity on the merged blob', () => {
    const round = toPreferencesForm(
      fromPreferencesForm({ ...DEFAULT_PREFERENCES, displayName: 'ada' }),
    );
    expect(round).toEqual({ ...DEFAULT_PREFERENCES, displayName: 'ada' });
  });
});
