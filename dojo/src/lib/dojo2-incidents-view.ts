// Pure presentation helpers for the lead confidentiality-incidents console
// (ScrIncidents, mockup). Side-effect-free (level in → token classes out) so the
// severity + state tones unit-test without a DOM and the screen stays
// declarative. Token-native ports of the mockup's `K2_SEV` / `K2_ISTATE` maps —
// colour is a named-token utility CLASS, never a raw oklch.

/** The token tone for a severity chip. */
export interface SeverityTone {
	text: string;
	soft: string;
	edge: string;
}

const SEVERITY_TONES: Record<string, SeverityTone> = {
	high: { text: 'text-danger', soft: 'bg-danger-soft', edge: 'border-danger-soft' },
	medium: { text: 'text-warning', soft: 'bg-warning-soft', edge: 'border-warning-soft' }
};

const NEUTRAL_SEVERITY: SeverityTone = {
	text: 'text-ink-mute',
	soft: 'bg-paper-mute',
	edge: 'border-paper-edge'
};

/** Token classes for an incident severity (mockup `K2_SEV`). high → danger,
 *  medium → warning, everything else → neutral ink. */
export function severityTone(severity: string): SeverityTone {
	return SEVERITY_TONES[severity] ?? NEUTRAL_SEVERITY;
}

const STATE_TONES: Record<string, string> = {
	contained: 'text-warning',
	resolved: 'text-success',
	open: 'text-danger'
};

/** The foreground token class for an incident state dot + label (mockup
 *  `K2_ISTATE`). contained → warning, resolved → success, open → danger;
 *  anything else reads as muted ink. */
export function stateToneClass(state: string): string {
	return STATE_TONES[state] ?? 'text-ink-mute';
}
