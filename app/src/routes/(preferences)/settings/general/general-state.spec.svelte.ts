import { describe, it, expect, vi, afterEach } from 'vitest';
import { PreferencesState } from './general-state.svelte';
import { DEFAULT_PREFERENCES } from '../preferences-form.js';

const read = (over: Partial<typeof DEFAULT_PREFERENCES> = {}, extra: { error?: string | null; prefilled?: boolean } = {}) => ({
  prefs: { ...DEFAULT_PREFERENCES, displayName: 'Ada', ...over },
  error: extra.error ?? null,
  prefilled: extra.prefilled ?? false,
});

afterEach(() => vi.useRealTimers());

describe('PreferencesState', () => {
  it('holds the read preferences', () => {
    const s = new PreferencesState(read());
    expect(s.prefs.displayName).toBe('Ada');
    expect(s.loadError).toBeNull();
  });

  it('never saves a form it could not read (#288)', async () => {
    const s = new PreferencesState({ prefs: null, error: 'Service Unavailable', prefilled: false });
    const save = vi.fn();
    await s.persist(save);
    expect(save).not.toHaveBeenCalled();
    expect(s.loadError).toBe('Service Unavailable');
  });

  it('saves, says saved, then settles back to idle', async () => {
    vi.useFakeTimers();
    const s = new PreferencesState(read());
    const save = vi.fn().mockResolvedValue(undefined);
    await s.persist(save);
    expect(save).toHaveBeenCalledOnce();
    expect(s.saveStatus).toBe('saved');
    vi.advanceTimersByTime(1600);
    expect(s.saveStatus).toBe('idle');
  });

  it('says why a save failed', async () => {
    const s = new PreferencesState(read());
    await s.persist(vi.fn().mockRejectedValue(new Error('daemon down')));
    expect(s.saveStatus).toBe('error');
    expect(s.saveError).toBe('daemon down');
  });

  it('saves the toggles only when one changed', async () => {
    const s = new PreferencesState(read());
    const save = vi.fn().mockResolvedValue(undefined);
    await s.saveIfTogglesChanged(save);
    expect(save).not.toHaveBeenCalled();
    s.prefs.showWelcome = !s.prefs.showWelcome;
    await s.saveIfTogglesChanged(save);
    await s.saveIfTogglesChanged(save);
    expect(save).toHaveBeenCalledOnce();
  });

  it('saves a prefilled display name once', async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    await new PreferencesState(read()).savePrefilled(save);
    expect(save).not.toHaveBeenCalled();
    await new PreferencesState(read({}, { prefilled: true })).savePrefilled(save);
    expect(save).toHaveBeenCalledOnce();
  });

  it('a re-read replaces the form without counting as a change', async () => {
    const s = new PreferencesState({ prefs: null, error: 'boom', prefilled: false });
    s.setRead(read({ showWelcome: !DEFAULT_PREFERENCES.showWelcome }));
    expect(s.loadError).toBeNull();
    const save = vi.fn().mockResolvedValue(undefined);
    await s.saveIfTogglesChanged(save);
    expect(save).not.toHaveBeenCalled();
  });
});
