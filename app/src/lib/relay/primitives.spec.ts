import { describe, it, expect } from 'vitest';
import {
  PILL_TONES,
  ROLE_STATUSES,
  pillClass,
  roleMarkClass,
  tagClass,
  tagText,
  tagTitle,
} from './primitives.js';

// COMPONENTS.md §3a (Pill, Tag, RoleMark): the class vocabulary each primitive
// draws. The component specs pin the rendered fixtures; this spec pins the
// mapping itself, so a wrong tone, radius or type stop fails on data rather
// than inside a DOM snapshot.

/** Every token a class in this file may reference. Sourced from rokkit's
 *  zen-sumi preflight plus the app's own `overrides` (rokkit.config.js); a name
 *  that is not emitted falls back to nothing, which is why it is pinned here. */
const KNOWN_TOKENS = new Set([
  'paper',
  'paper-soft',
  'paper-mute',
  'paper-edge',
  'paper-edge-strong',
  'ink',
  'ink-soft',
  'ink-mute',
  'ink-faint',
  'accent',
  'accent-soft',
  'success',
  'success-soft',
  'warning',
  'warning-soft',
  'danger',
  'danger-soft',
]);

function tokensIn(cls: string): string[] {
  return [...cls.matchAll(/var\(--([a-z-]+)\)/g)].map((m) => m[1]);
}

describe('the relay primitive class vocabulary', () => {
  it('names only tokens the app emits, and no literal colour', () => {
    const all = [
      ...PILL_TONES.map(pillClass),
      tagClass('family'),
      tagClass('model'),
      tagClass('host'),
      roleMarkClass('coordinator', 'active', 24).frame,
      roleMarkClass('contributor', 'active', 24).frame,
      ...ROLE_STATUSES.map((s) => roleMarkClass('contributor', s, 30).frame),
      ...ROLE_STATUSES.map((s) => roleMarkClass('contributor', s, 30).glyph),
    ].join(' ');
    for (const name of tokensIn(all)) expect(KNOWN_TOKENS).toContain(name);
    expect(all).not.toMatch(/#[0-9a-f]{3,8}\b/i);
    expect(all).not.toMatch(/\b(rgb|hsl|oklch)\(/);
  });
});

describe('Pill (§3a)', () => {
  it('is 11px, 1/8 padding, full radius, nowrap, 1px border', () => {
    for (const tone of PILL_TONES) {
      const cls = pillClass(tone);
      expect(cls).toContain('text-xs');
      expect(cls).toContain('px-2');
      expect(cls).toContain('py-px');
      expect(cls).toContain('rounded-full');
      expect(cls).toContain('whitespace-nowrap');
      expect(cls).toMatch(/\bborder\b/);
      expect(cls).toMatch(/\bborder-solid\b/);
    }
  });

  // The mockup's tint(role) is the tone at 16% over the paper, so it darkens with the
  // page: the app's -soft tokens stay light in dark mode, under the pill's light text
  // (unreadable on the #422 bench).
  // Mixed in OKLab, not OKLCH: OKLCH interpolates the hue, and over the app's warm paper
  // a 16% red came out amber and a green olive (#423 bench). OKLab blends without
  // rotating the hue, so each tone keeps its colour.
  it('fills with the tone at 16% over the paper, and borders it at 35% — mixed in OKLab', () => {
    for (const tone of ['accent', 'warning', 'danger', 'success'] as const) {
      const cls = pillClass(tone);
      expect(cls).toContain(`bg-[color-mix(in_oklab,var(--${tone})_16%,var(--paper))]`);
      expect(cls).not.toContain(`bg-${tone}-soft`);
      expect(cls).toContain(`border-[color-mix(in_oklab,var(--${tone})_35%,var(--paper))]`);
      expect(cls).not.toContain('in_oklch');
    }
  });

  it('draws neutral on the mute surface, because the app has no neutral tone', () => {
    const cls = pillClass('neutral');
    expect(cls).toContain('bg-paper-mute');
    expect(cls).toContain('border-paper-edge');
    expect(cls).not.toContain('bg-neutral');
  });
});

describe('Tag (§3a)', () => {
  it('family: mute fill, ink text, 2/8 padding', () => {
    const cls = tagClass('family');
    expect(cls).toContain('bg-paper-mute');
    expect(cls).toContain('text-ink');
    expect(cls).toContain('py-0.5');
    expect(cls).toContain('px-2');
    expect(cls).toContain('rounded-full');
    expect(cls).not.toContain('font-mono');
  });

  it('model: mono 11, 1px edge, soft text', () => {
    const cls = tagClass('model');
    expect(cls).toContain('font-mono');
    expect(cls).toContain('border-paper-edge');
    expect(cls).toContain('text-ink-soft');
    expect(cls).toContain('py-0.5');
  });

  it('host: 0/8 padding, the strong edge, soft text', () => {
    const cls = tagClass('host');
    expect(cls).toContain('py-0');
    expect(cls).toContain('px-2');
    expect(cls).toContain('border-paper-edge-strong');
    expect(cls).toContain('text-ink-soft');
  });

  it('host reads "in <editor>" and explains the editor in its title', () => {
    expect(tagText('host', '', 'Cursor')).toBe('in Cursor');
    expect(tagTitle('host', 'Cursor')).toBe(
      'Cursor is the editor; the assistant inside it does the work.',
    );
    expect(tagText('family', 'Muse', '')).toBe('Muse');
    expect(tagText('model', 'claude-opus-5-5', '')).toBe('claude-opus-5-5');
    expect(tagTitle('family', 'Cursor')).toBeUndefined();
  });
});

describe('RoleMark (§3a)', () => {
  it('lane (24): a 24px circle; the coordinator is accent-tinted, a contributor is paper', () => {
    const coord = roleMarkClass('coordinator', 'active', 24);
    expect(coord.frame).toContain('w-6');
    expect(coord.frame).toContain('h-6');
    expect(coord.frame).toContain('rounded-full');
    expect(coord.frame).toContain('border-paper-edge');
    expect(coord.frame).toContain('bg-[color-mix(in_oklab,var(--accent)_20%,var(--paper))]');
    expect(coord.glyph).toContain('text-xs');
    expect(coord.glyph).toContain('text-accent');
    expect(coord.title).toBe('Coordinator');

    const contrib = roleMarkClass('contributor', 'active', 24);
    expect(contrib.frame).toContain('bg-paper');
    expect(contrib.title).toBe('Contributor');
  });

  it('crew (30): a 30px circle filled with the status tone at 14%, bordered at 45%', () => {
    const cases = {
      active: ['success', 'Active'],
      idle: ['ink-mute', 'Idle'],
      not_responding: ['warning', 'Not responding'],
    } as const;
    for (const status of ROLE_STATUSES) {
      const [tone, label] = cases[status];
      const mark = roleMarkClass('contributor', status, 30);
      expect(mark.frame).toContain('w-[30px]');
      expect(mark.frame).toContain('h-[30px]');
      expect(mark.frame).toContain(`bg-[color-mix(in_oklab,var(--${tone})_14%,var(--paper))]`);
      expect(mark.frame).toContain(`border-[color-mix(in_oklab,var(--${tone})_45%,var(--paper))]`);
      expect(mark.glyph).toContain('text-sm');
      expect(mark.glyph).toContain(`text-${tone}`);
      expect(mark.title).toBe(`Contributor · ${label}`);
    }
  });

  it('never reads status as colour alone: the title says role and status', () => {
    expect(roleMarkClass('coordinator', 'active', 30).title).toBe('Coordinator · Active');
    expect(roleMarkClass('coordinator', 'not_responding', 30).title).toBe(
      'Coordinator · Not responding',
    );
  });
});
