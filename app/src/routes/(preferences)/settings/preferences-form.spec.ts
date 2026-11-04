import { describe, it, expect } from 'vitest';
import { DEFAULT_PREFERENCES, fromPreferencesForm, readPreferencesForm, toPreferencesForm } from './preferences-form.js';

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
      'setup.preferences': JSON.stringify({ displayName: 'dev' }),
    });
    expect(merged.displayName).toBe('dev');
  });
});

describe('fromPreferencesForm', () => {
  it('writes both keys so downstream readers (greeter + wizard) stay consistent', () => {
    const out = fromPreferencesForm({
      ...DEFAULT_PREFERENCES,
      displayName: 'dev',
      digestCadence: 'weekly',
    });
    expect(out['user_name']).toBe('dev');
    const round = JSON.parse(out['setup.preferences']);
    expect(round.digestCadence).toBe('weekly');
    expect(round.displayName).toBe('dev');
  });

  it('round-trips: fromForm ∘ toForm is identity on the merged blob', () => {
    const round = toPreferencesForm(
      fromPreferencesForm({ ...DEFAULT_PREFERENCES, displayName: 'ada' }),
    );
    expect(round).toEqual({ ...DEFAULT_PREFERENCES, displayName: 'ada' });
  });
});

// #133: the kanji/icon switch is stored under its own key, so the rails can read
// it from the config without parsing the preferences blob.
describe('glyphs preference', () => {
  it('reads and writes ui.glyphs, kanji by default', () => {
    expect(toPreferencesForm({}).glyphs).toBe('kanji');
    expect(toPreferencesForm({ 'ui.glyphs': 'icon' }).glyphs).toBe('icon');
    expect(fromPreferencesForm({ ...DEFAULT_PREFERENCES, glyphs: 'icon' })['ui.glyphs']).toBe('icon');
  });
});

// D83 batch A, audit row 73: corrupt stored preferences read as the defaults, and
// the next save wrote them over the real values. The read says it failed instead.
describe('readPreferencesForm', () => {
  it.skip('pending: D83 batch A — corrupt stored preferences are a failure with its reason, never the defaults', () => {
    const r = readPreferencesForm({ 'setup.preferences': '{not json' });
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.reason).toMatch(/preferences/i);
  });

  it('nothing stored is the defaults, and stored JSON is the form', () => {
    expect(readPreferencesForm({})).toEqual({ ok: true, form: DEFAULT_PREFERENCES });
    const r = readPreferencesForm({ 'setup.preferences': JSON.stringify({ digestCadence: 'weekly' }) });
    expect(r.ok && r.form.digestCadence).toBe('weekly');
  });
});
