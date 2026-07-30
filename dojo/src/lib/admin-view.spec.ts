import { describe, expect, it } from 'vitest';
import {
	actionLabel,
	actionToneClass,
	attributionLabel,
	attributionToneClass,
	clockTime,
	errorRateToneClass,
	providerKanji,
	providerLabel,
	queueDepthToneClass,
	relativeAge,
	retentionLabel,
	roleLabel,
	roleToneClass,
	shortId
} from '$lib/admin-view';

describe('roleLabel / roleToneClass', () => {
	it('maps the known Dōjō roles to labels', () => {
		expect(roleLabel('admin')).toBe('Org admin');
		expect(roleLabel('maintainer')).toBe('Maintainer');
		expect(roleLabel('lead')).toBe('Lead');
		expect(roleLabel('contributor')).toBe('Contributor');
	});

	it('titleizes an unknown role rather than throwing', () => {
		expect(roleLabel('read_only')).toBe('Read only');
	});

	it('tones admin as accent and an unknown role as faint', () => {
		expect(roleToneClass('admin')).toBe('text-accent');
		expect(roleToneClass('maintainer')).toBe('text-ink');
		expect(roleToneClass('mystery')).toBe('text-ink-faint');
	});
});

describe('providerLabel / providerKanji', () => {
	it('labels the three auth methods', () => {
		expect(providerLabel('sso')).toBe('SSO · OIDC/SAML');
		expect(providerLabel('github_oauth')).toBe('GitHub OAuth');
		expect(providerLabel('device_code')).toBe('Device code');
	});

	it('gives each provider a glyph and defaults unknown to the key kanji', () => {
		expect(providerKanji('sso')).toBe('鍵');
		expect(providerKanji('github_oauth')).toBe('猫');
		expect(providerKanji('what')).toBe('鍵');
	});
});

describe('attribution', () => {
	it('labels + tones each mode (anonymous reads accent)', () => {
		expect(attributionLabel('named')).toBe('Named');
		expect(attributionLabel('anonymous')).toBe('Anonymous');
		expect(attributionToneClass('anonymous')).toBe('text-accent');
		expect(attributionToneClass('named')).toBe('text-ink-soft');
	});
});

describe('retentionLabel', () => {
	it('formats windows and indefinite', () => {
		expect(retentionLabel(null)).toBe('—');
		expect(retentionLabel(0)).toBe('none');
		expect(retentionLabel(30)).toBe('1mo');
		expect(retentionLabel(365)).toBe('1y');
		expect(retentionLabel(45)).toBe('45d');
	});
});

describe('health tones', () => {
	it('warns on any error in the last hour, else success', () => {
		expect(errorRateToneClass(0)).toBe('text-success');
		expect(errorRateToneClass(3)).toBe('text-warning');
	});

	it('accents a backed-up queue, else muted', () => {
		expect(queueDepthToneClass(0)).toBe('text-ink-mute');
		expect(queueDepthToneClass(2)).toBe('text-accent');
	});
});

describe('audit action label + tone', () => {
	it('labels known actions and de-underscores unknown', () => {
		expect(actionLabel('policy_edited')).toBe('Policy edited');
		expect(actionLabel('member_added')).toBe('Member added');
		expect(actionLabel('some_new_thing')).toBe('Some new thing');
	});

	it('tones by action family', () => {
		expect(actionToneClass('identity_deleted')).toBe('text-warning');
		expect(actionToneClass('member_added')).toBe('text-success');
		expect(actionToneClass('engagement_created')).toBe('text-success');
		expect(actionToneClass('role_changed')).toBe('text-accent');
		expect(actionToneClass('policy_edited')).toBe('text-accent');
		expect(actionToneClass('mystery')).toBe('text-ink-soft');
	});
});

describe('timestamps', () => {
	const now = new Date('2026-07-13T12:00:00Z');

	it('relativeAge is compact and null-safe', () => {
		expect(relativeAge(null, now)).toBe('—');
		expect(relativeAge('nonsense', now)).toBe('—');
		expect(relativeAge('2026-07-13T11:59:30Z', now)).toBe('just now');
		expect(relativeAge('2026-07-13T11:45:00Z', now)).toBe('15m');
		expect(relativeAge('2026-07-13T09:00:00Z', now)).toBe('3h');
		expect(relativeAge('2026-07-10T12:00:00Z', now)).toBe('3d');
	});

	it('clockTime reads the HH:MM slice, timezone-stable', () => {
		expect(clockTime('2026-07-13T09:42:07Z')).toBe('09:42');
		expect(clockTime(null)).toBe('—');
		expect(clockTime('garbage')).toBe('—');
	});

	it('shortId truncates and is null-safe', () => {
		expect(shortId(null)).toBe('—');
		expect(shortId('abcdef')).toBe('abcdef');
		expect(shortId('0123456789abcdef')).toBe('01234567');
		expect(shortId('0123456789abcdef', 4)).toBe('0123');
	});
});
