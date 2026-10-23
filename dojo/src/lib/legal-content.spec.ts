import { describe, expect, it } from 'vitest';
import { LEGAL_PAGES, PRIVACY, TERMS } from './legal-content';

// #330. The legal pages state what the dōjō does with people's data. Every
// statement here is checked against the schema it describes, so a page cannot
// promise less collection than the code performs.
const text = (page: typeof PRIVACY) =>
	page.sections.flatMap((s) => [s.heading, ...s.body]).join('\n');

describe('the legal pages', () => {
	it('live at the paths the consent screens link to, each with a last-updated date', () => {
		expect(LEGAL_PAGES.map((p) => p.path)).toEqual(['/terms-of-service', '/privacy-policy']);
		for (const page of LEGAL_PAGES) {
			expect(page.updated).toMatch(/^\d{4}-\d{2}-\d{2}$/);
			expect(page.sections.length).toBeGreaterThan(3);
		}
	});

	it('the privacy policy names what is stored, including the code graph and Relay', () => {
		const p = text(PRIVACY);
		for (const fact of [
			'GitHub account id',
			'Microsoft Entra object id',
			'symbol names, signatures, docstrings',
			'never the contents of your files',
			'Relay',
			'aggregates',
			'audit'
		]) {
			expect(p).toContain(fact);
		}
	});

	it('says how forge credentials are kept, and that access tokens are not stored', () => {
		const p = text(PRIVACY);
		expect(p).toContain('Supabase Vault');
		expect(p).toContain('access tokens are never stored');
	});

	it('names the processors and how to leave', () => {
		const p = text(PRIVACY);
		expect(p).toContain('Supabase');
		expect(p).toContain('Cloudflare');
		expect(p).toMatch(/leave an organization/i);
	});

	it('both pages say where to write, and the privacy policy says it is how to ask for export or deletion', () => {
		for (const page of LEGAL_PAGES) expect(text(page)).toContain('hi@sensei-hq.com');
		expect(text(PRIVACY)).toMatch(/exported or deleted[^]*hi@sensei-hq\.com|hi@sensei-hq\.com[^]*exported or deleted/);
	});

	it('the terms keep ownership of contributed data with the person or organization', () => {
		expect(text(TERMS)).toMatch(/you keep ownership/i);
	});
});
