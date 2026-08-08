// Observatory · Today — view derivations and copy.
// The daemon owns the maturity gate and every user-facing string (koan,
// insights, adopted). This module only projects the wire numbers into the
// header chip / strip and maps discriminators (insight tone, hero
// provenance) to tokens — no screen-side decisions about what to teach.
import type { ObservatoryFtr, ObservatoryInsight, ObservatoryTodayHero } from '$lib/types.js';

export type FtrDirection = 'up' | 'down' | 'flat';

export interface FtrChip {
  /** 14-day FTR as a whole-number percentage (0–100). */
  value: number;
  /** Signed change vs the prior 14 days, in percentage points. */
  delta: number;
  direction: FtrDirection;
  /** Arrow glyph for the direction: ↑ / ↓ / →. */
  arrow: string;
  /** Text-color utility for the delta glyph. */
  toneClass: string;
}

/**
 * Project the raw FTR wire numbers into the header chip. `delta` reflects
 * `ftr14d - ftr14dPrev` (spec), so the arrow reads up / down / flat.
 */
export function ftrChip(ftr: ObservatoryFtr): FtrChip {
  const value = Math.round(ftr.ftr14d * 100);
  const delta = Math.round((ftr.ftr14d - ftr.ftr14dPrev) * 100);
  const direction: FtrDirection = delta > 0 ? 'up' : delta < 0 ? 'down' : 'flat';
  const arrow = direction === 'up' ? '↑' : direction === 'down' ? '↓' : '→';
  const toneClass =
    direction === 'up' ? 'text-success' : direction === 'down' ? 'text-warning' : 'text-ink-soft';
  return { value, delta, direction, arrow, toneClass };
}

/** Bars for the FtrStrip component, one per point of the 14-day trend. */
export function ftrBars(ftr: ObservatoryFtr): { rate: number }[] {
  return ftr.ftrTrend.map((rate) => ({ rate }));
}

export interface InsightToneClasses {
  /** Kanji glyph color. */
  glyph: string;
  /** Tag pill background + text. */
  tag: string;
}

/** Map an insight's tone discriminator to named-token utility classes. */
export function insightToneClasses(tone: ObservatoryInsight['tone']): InsightToneClasses {
  switch (tone) {
    // Tag chips sit on `bg-paper-mute` (a paper token that FLIPS dark for dark
    // mode) with a tone-coloured border + text — NOT a `*-soft` fill. The status
    // softs are single-pole (stay pale ~0.94L in both modes), so tone-coloured
    // text on them is near-invisible in dark mode (the "[high] barely visible" bug).
    case 'warn':
      return { glyph: 'text-warning', tag: 'bg-paper-mute text-warning border border-warning' };
    case 'good':
      return { glyph: 'text-success', tag: 'bg-paper-mute text-success border border-success' };
    default:
      // 'mute' → ink-mute is the named token for intentionally suppressed
      // content (matches the card body's text-ink-mute); ink-soft is for
      // secondary text and would render the muted glyph lighter than its body.
      return { glyph: 'text-ink-mute', tag: 'bg-paper-mute text-ink-mute' };
  }
}

/**
 * The pinned header greeting, e.g. "Good evening." Falls back to a neutral
 * line when the daemon sends no greeting. Sentence case, never title case.
 */
export function greetingLine(greeting: string): string {
  const g = greeting.trim();
  return g ? `${g}.` : 'Welcome back.';
}

/**
 * Join the hero's provenance parts (session ids + relative "noticed") with
 * " · ", dropping empties so an absent source leaves no dangling separator.
 * Returns "" when nothing is known — the caller then renders nothing.
 */
export function heroProvenance(hero: Pick<ObservatoryTodayHero, 'source' | 'noticed'>): string {
  return [hero.source, hero.noticed]
    .map((p) => p?.trim() ?? '')
    .filter(Boolean)
    .join(' · ');
}
