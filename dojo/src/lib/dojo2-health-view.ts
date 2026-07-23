// Pure presentation helpers for the admin Health / Monitor console (ScrHealth,
// mockup). Side-effect-free (signal tone key / bar series in → token class or
// height % out) so the tones and the contributions-vs-approvals bar geometry
// unit-test without a DOM and the screen stays declarative. The mockup stored
// raw `var(--*)` strings; here each tone is a named-token utility CLASS.

import type { KitHealthWeek } from './components/kit/types';

const SIGNAL_TONES: Record<string, string> = {
	accent: 'text-accent',
	success: 'text-success',
	warning: 'text-warning',
	ink: 'text-ink',
	danger: 'text-danger'
};

/** The foreground token class for a health signal's tone key (the mockup stored
 *  `var(--accent)` etc.; the ported fixture stores the bare key). Unknown keys
 *  read as neutral ink. */
export function signalToneClass(tone: string): string {
	return SIGNAL_TONES[tone] ?? 'text-ink';
}

/** The foreground token class for an alert glyph by severity (mockup: warning ⇒
 *  warning, anything else ⇒ success). */
export function alertToneClass(sev: string): string {
	return sev === 'warning' ? 'text-warning' : 'text-success';
}

/** The tallest bar value across the whole contributions-vs-approvals series —
 *  the shared scale every bar's height is a fraction of (mockup
 *  `Math.max(...flatMap([c, a]))`). 0 for an empty series (avoids /0). */
export function barMax(weeks: KitHealthWeek[]): number {
	return weeks.reduce((m, w) => Math.max(m, w.c, w.a), 0);
}

/** A bar's height as a whole-number percent of the shared scale (mockup
 *  `v / max * 100 + "%"`), clamped to 0 when the scale is 0. */
export function barPct(value: number, max: number): number {
	return max > 0 ? Math.round((value / max) * 100) : 0;
}
