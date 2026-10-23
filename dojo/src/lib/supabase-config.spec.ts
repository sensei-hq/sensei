// @vitest-environment node
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

// The local Supabase Auth configuration the sign-in page depends on
// (supabase/config.toml). Read as text: the keys asserted here are flat
// `key = value` lines under one table header, and the CLI itself parses and
// validates the file on every `supabase status`.
const TOML = readFileSync(new URL('../../../supabase/config.toml', import.meta.url), 'utf8');

/** The `key = value` lines of one `[table]`, values as written (quotes kept). */
function table(name: string): Record<string, string> {
	const out: Record<string, string> = {};
	let inside = false;
	for (const raw of TOML.split('\n')) {
		const line = raw.trim();
		if (line.startsWith('[')) {
			inside = line === `[${name}]`;
			continue;
		}
		if (!inside || !line || line.startsWith('#')) continue;
		const at = line.indexOf('=');
		if (at > 0) out[line.slice(0, at).trim()] = line.slice(at + 1).trim();
	}
	return out;
}

// Sign in with Microsoft (#308 R5c, D45): Supabase Auth's Azure provider, from
// the one multi-tenant app registration R5b's connect also uses.
describe('supabase config — the Azure provider', () => {
	const azure = table('auth.external.azure');

	it('is enabled exactly as GitHub is: on, with its client id and secret from the environment', () => {
		// With the variables unset the CLI warns and starts anyway (it keeps the
		// literal `env(…)` text), as it does for GitHub — so a machine without the
		// app registration still runs `supabase start`; only the Microsoft button
		// fails, at Microsoft.
		expect(table('auth.external.github').enabled).toBe('true');
		expect(azure.enabled).toBe('true');
		expect(azure.client_id).toBe('"env(SUPABASE_AUTH_EXTERNAL_AZURE_CLIENT_ID)"');
		expect(azure.secret).toBe('"env(SUPABASE_AUTH_EXTERNAL_AZURE_SECRET)"');
	});

	it('signs in against the multi-tenant `organizations` authority', () => {
		// GoTrue builds `{url}/oauth2/v2.0/authorize` and accepts any Entra issuer
		// for `organizations` (supabase/auth internal/api/provider/azure.go). Work or
		// school accounts only: personal Microsoft accounts cannot use Azure DevOps.
		expect(azure.url).toBe('"https://login.microsoftonline.com/organizations"');
	});

	it('returns from Microsoft to localhost, which Microsoft accepts over plain http (127.0.0.1 it does not)', () => {
		expect(azure.redirect_uri).toBe('"http://localhost:54321/auth/v1/callback"');
	});
});

describe('supabase config — where a sign-in may land', () => {
	const allowed = (): string[] => JSON.parse(table('auth').additional_redirect_urls ?? '[]');

	it('allows the dōjō at localhost, the origin a Microsoft sign-in runs from locally', () => {
		expect(allowed()).toEqual(expect.arrayContaining(['http://localhost:5173', 'http://localhost:5173/**', 'http://localhost:5175/**']));
	});

	it('keeps every origin it allowed before', () => {
		expect(allowed()).toEqual(
			expect.arrayContaining([
				'http://127.0.0.1:5175',
				'http://localhost:5175',
				'http://127.0.0.1:5175/org/**',
				'http://127.0.0.1:5173/org/**',
				'http://127.0.0.1:7744/api/auth/callback',
				'http://localhost:7744/api/auth/callback'
			])
		);
	});
});
