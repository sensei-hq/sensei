import { describe, expect, it } from 'vitest';
import { severityTone, stateToneClass } from './incidents-view';

// Pure tone helpers for the lead incidents console. No DOM — the severity chip
// tone and the state-dot tone map level → token class on their own.

describe('severityTone — token classes for the severity chip', () => {
	it('tints high severity danger', () => {
		expect(severityTone('high').text).toBe('text-danger');
		expect(severityTone('high').soft).toBe('bg-danger-soft');
	});

	it('tints medium severity warning', () => {
		expect(severityTone('medium').text).toBe('text-warning');
		expect(severityTone('medium').soft).toBe('bg-warning-soft');
	});

	it('falls back to neutral for an unknown severity', () => {
		expect(severityTone('trivial').text).toBe('text-ink-mute');
	});
});

describe('stateToneClass — the state dot + label tone', () => {
	it('maps contained → warning, resolved → success, open → danger', () => {
		expect(stateToneClass('contained')).toBe('text-warning');
		expect(stateToneClass('resolved')).toBe('text-success');
		expect(stateToneClass('open')).toBe('text-danger');
	});

	it('falls back to muted ink for an unknown state', () => {
		expect(stateToneClass('archived')).toBe('text-ink-mute');
	});
});
