import { describe, it, expect } from 'vitest';
import {
  ftrChip,
  ftrBars,
  insightToneClasses,
  greetingLine,
  heroProvenance,
} from './today-view.svelte.js';
import type { ObservatoryFtr } from '$lib/types.js';

const ftr = (over: Partial<ObservatoryFtr> = {}): ObservatoryFtr => ({
  ftr14d: 0.5,
  ftr14dPrev: 0.7333333333333333,
  ftrTrend: [0.1, 0.2, 0.3],
  sessions7d: 10,
  ...over,
});

describe('ftrChip', () => {
  it('rounds ftr14d to a whole-number percentage', () => {
    expect(ftrChip(ftr({ ftr14d: 0.784 })).value).toBe(78);
  });

  it('computes a negative delta + down arrow when the rate fell', () => {
    const chip = ftrChip(ftr({ ftr14d: 0.5, ftr14dPrev: 0.7333333333333333 }));
    expect(chip.delta).toBe(-23);
    expect(chip.direction).toBe('down');
    expect(chip.arrow).toBe('↓');
    expect(chip.toneClass).toBe('text-warning');
  });

  it('computes a positive delta + up arrow when the rate rose', () => {
    const chip = ftrChip(ftr({ ftr14d: 0.78, ftr14dPrev: 0.72 }));
    expect(chip.delta).toBe(6);
    expect(chip.direction).toBe('up');
    expect(chip.arrow).toBe('↑');
    expect(chip.toneClass).toBe('text-success');
  });

  it('reads flat with a → arrow when the rate is unchanged', () => {
    const chip = ftrChip(ftr({ ftr14d: 0.6, ftr14dPrev: 0.6 }));
    expect(chip.delta).toBe(0);
    expect(chip.direction).toBe('flat');
    expect(chip.arrow).toBe('→');
    expect(chip.toneClass).toBe('text-ink-soft');
  });
});

describe('ftrBars', () => {
  it('maps the trend into one { rate } point per day', () => {
    expect(ftrBars(ftr({ ftrTrend: [0.2, 0.4] }))).toEqual([{ rate: 0.2 }, { rate: 0.4 }]);
  });

  it('returns an empty list for an empty trend', () => {
    expect(ftrBars(ftr({ ftrTrend: [] }))).toEqual([]);
  });
});

describe('insightToneClasses', () => {
  it('maps warn to warning tokens', () => {
    expect(insightToneClasses('warn')).toEqual({ glyph: 'text-warning', tag: 'bg-paper-mute text-warning border border-warning' });
  });

  it('maps good to success tokens', () => {
    expect(insightToneClasses('good')).toEqual({ glyph: 'text-success', tag: 'bg-paper-mute text-success border border-success' });
  });

  it('maps mute to neutral ink tokens', () => {
    expect(insightToneClasses('mute')).toEqual({ glyph: 'text-ink-mute', tag: 'bg-paper-mute text-ink-mute' });
  });
});

describe('greetingLine', () => {
  it('appends a period to the daemon greeting', () => {
    expect(greetingLine('Good evening')).toBe('Good evening.');
  });

  it('falls back to a neutral line when the greeting is empty', () => {
    expect(greetingLine('')).toBe('Welcome back.');
    expect(greetingLine('   ')).toBe('Welcome back.');
  });
});

describe('heroProvenance', () => {
  it('joins source and noticed with a middot', () => {
    expect(heroProvenance({ source: 'from s-2891', noticed: 'noticed 2 days ago' })).toBe(
      'from s-2891 · noticed 2 days ago',
    );
  });

  it('drops an empty source so there is no dangling separator', () => {
    expect(heroProvenance({ source: '', noticed: 'noticed 2 days ago' })).toBe('noticed 2 days ago');
  });

  it('returns an empty string when nothing is known', () => {
    expect(heroProvenance({ source: '', noticed: '' })).toBe('');
    expect(heroProvenance({ source: '  ', noticed: '  ' })).toBe('');
  });
});
