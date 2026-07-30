// Pure presentation helpers for the admin-console screens (R10).
//
// Side-effect-free (data in => display value out) so they unit-test without a DOM
// or a live backend, and so the screens stay declarative. Colour is a token
// utility CLASS name (text-accent / text-success / ...) per the console design
// system — never a raw hex/oklch.

// ── member role => label + token tone ────────────────────────────────────────

const ROLE_LABELS: Record<string, string> = {
	admin: 'Org admin',
	maintainer: 'Maintainer',
	lead: 'Lead',
	contributor: 'Contributor'
};

/** Human label for a Dōjō member role; falls back to a titled raw value. */
export function roleLabel(role: string): string {
	return ROLE_LABELS[role] ?? titleize(role);
}

/** The token tone class for a role (mockup roleTone: admin=accent ... read-only=faint). */
export function roleToneClass(role: string): string {
	switch (role) {
		case 'admin':
			return 'text-accent';
		case 'maintainer':
			return 'text-ink';
		case 'contributor':
		case 'lead':
			return 'text-ink-soft';
		default:
			return 'text-ink-faint';
	}
}

/** The four selectable member roles, in privilege order (for a role picker). */
export const MEMBER_ROLES: readonly string[] = [
	'contributor',
	'maintainer',
	'lead',
	'admin'
] as const;

// ── auth method => label + kanji ─────────────────────────────────────────────

const PROVIDER_LABELS: Record<string, string> = {
	sso: 'SSO · OIDC/SAML',
	github_oauth: 'GitHub OAuth',
	device_code: 'Device code'
};

/** Human label for an identity provider (auth method). */
export function providerLabel(provider: string): string {
	return PROVIDER_LABELS[provider] ?? titleize(provider);
}

const PROVIDER_KANJI: Record<string, string> = {
	sso: '鍵',
	github_oauth: '猫',
	device_code: '端'
};

/** A kanji glyph for an identity provider row marker (defaults to 鍵 "key"). */
export function providerKanji(provider: string): string {
	return PROVIDER_KANJI[provider] ?? '鍵';
}

/** The three selectable identity providers (for a provider picker). */
export const AUTH_METHODS: readonly string[] = ['sso', 'github_oauth', 'device_code'] as const;

// ── attribution mode => label + tone ─────────────────────────────────────────

const ATTRIBUTION_LABELS: Record<string, string> = {
	named: 'Named',
	anonymous: 'Anonymous'
};

/** Human label for a policy attribution mode. */
export function attributionLabel(mode: string): string {
	return ATTRIBUTION_LABELS[mode] ?? titleize(mode);
}

/** Token tone for an attribution mode (anonymous reads as the accent/privacy mark). */
export function attributionToneClass(mode: string): string {
	return mode === 'anonymous' ? 'text-accent' : 'text-ink-soft';
}

/** The two selectable attribution modes (for a policy picker). Source-dereference
 * is always-on and not a mode. */
export const ATTRIBUTION_MODES: readonly string[] = ['named', 'anonymous'] as const;

/** A retention window in days => compact label (`—` for indefinite/null). */
export function retentionLabel(days: number | null): string {
	if (days == null) return '—';
	if (days === 0) return 'none';
	if (days % 365 === 0) return `${days / 365}y`;
	if (days % 30 === 0) return `${days / 30}mo`;
	return `${days}d`;
}

// ── health => status tone (mockup: 0 errors = success, else warning) ─────────

/** Token tone for the error-rate cell: any error in the last hour reads warning. */
export function errorRateToneClass(errorRate: number): string {
	return errorRate > 0 ? 'text-warning' : 'text-success';
}

/** Token tone for the queue-depth cell: a backed-up queue reads accent. */
export function queueDepthToneClass(queueDepth: number): string {
	return queueDepth > 0 ? 'text-accent' : 'text-ink-mute';
}

// ── audit action => label + tone (mockup DojoAudit evTone families) ──────────

const ACTION_LABELS: Record<string, string> = {
	member_added: 'Member added',
	role_changed: 'Role changed',
	identity_added: 'Identity added',
	identity_updated: 'Identity updated',
	identity_deleted: 'Identity deleted',
	policy_edited: 'Policy edited',
	policy_deleted: 'Policy deleted',
	engagement_created: 'Engagement created',
	engagement_updated: 'Engagement updated'
};

/** Human label for an audit action; falls back to a de-underscored raw value. */
export function actionLabel(action: string): string {
	return ACTION_LABELS[action] ?? titleize(action);
}

/**
 * Token tone for an audit action pill: destructive (delete) reads warning,
 * additive (add/create) reads success, changes read accent, else muted ink.
 */
export function actionToneClass(action: string): string {
	if (action.endsWith('_deleted')) return 'text-warning';
	if (action.endsWith('_added') || action.endsWith('_created')) return 'text-success';
	if (action.endsWith('_changed') || action.endsWith('_edited') || action.endsWith('_updated'))
		return 'text-accent';
	return 'text-ink-soft';
}

// ── timestamps ────────────────────────────────────────────────────────────────

/**
 * Compact "how long ago" for an ISO timestamp against `now` (default: current
 * time). Returns `just now`, `12m`, `5h`, `3d`. `null`/unparseable => `—`.
 * Injectable `now` keeps it deterministic under test.
 */
export function relativeAge(iso: string | null, now: Date = new Date()): string {
	if (!iso) return '—';
	const then = new Date(iso).getTime();
	if (Number.isNaN(then)) return '—';
	const secs = Math.max(0, Math.floor((now.getTime() - then) / 1000));
	if (secs < 60) return 'just now';
	const mins = Math.floor(secs / 60);
	if (mins < 60) return `${mins}m`;
	const hours = Math.floor(mins / 60);
	if (hours < 24) return `${hours}h`;
	const days = Math.floor(hours / 24);
	return `${days}d`;
}

/** The `HH:MM` clock portion of an ISO timestamp (`—` when unparseable/null). */
export function clockTime(iso: string | null): string {
	if (!iso) return '—';
	// The store emits `YYYY-MM-DDTHH:MM:SSZ`; read the HH:MM slice directly rather
	// than through Date so it's timezone-stable and testable.
	const m = /T(\d{2}:\d{2})/.exec(iso);
	return m ? m[1] : '—';
}

/** A short id/subject for a dense cell — the leading `n` chars (default 8). */
export function shortId(id: string | null, n = 8): string {
	if (!id) return '—';
	return id.length > n ? id.slice(0, n) : id;
}

// ── internal ──────────────────────────────────────────────────────────────────

/** `some_raw_value` / `sso` => `Some raw value` / `Sso`. Never throws. */
function titleize(s: string): string {
	if (!s) return '';
	const spaced = s.replace(/_/g, ' ');
	return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}
