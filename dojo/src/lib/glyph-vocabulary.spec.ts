import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'fs';
import { join } from 'path';
import { IDENTITY_KANJI, KANJI_ICONS } from '@sensei/glyphs';

// #133: the dōjō reads the same kanji→icon table as the desktop app
// (packages/glyphs). Every kanji the dōjō renders must be in it — or be an
// identity mark — and every icon must be an installed Solar Bold Duotone class.
const solar = new Set(
	Object.keys(JSON.parse(readFileSync('node_modules/@iconify-json/solar/icons.json', 'utf8')).icons)
);

function sources(dir: string): string[] {
	return readdirSync(dir).flatMap((name) => {
		const p = join(dir, name);
		if (statSync(p).isDirectory()) return name === 'paraglide' ? [] : sources(p);
		return /\.(svelte|ts)$/.test(p) && !p.includes('.spec.') ? [p] : [];
	});
}

const KANJI_SITE =
	/(?:kanji["']?\s*[:=]\s*["'{]?\s*["']?|class="kanji[^"]*">\s*|char=")(\p{Script=Han})/gu;

describe('the shared glyph vocabulary, from the dōjō', () => {
	it('covers every kanji the dōjō renders', () => {
		const missing = new Map<string, string>();
		for (const file of sources('src')) {
			for (const m of readFileSync(file, 'utf8').matchAll(KANJI_SITE)) {
				if (!KANJI_ICONS[m[1]] && !IDENTITY_KANJI.has(m[1])) missing.set(m[1], file.replace('src/', ''));
			}
		}
		expect(Object.fromEntries(missing)).toEqual({});
	});

	it('maps every kanji to an installed Solar Bold Duotone icon', () => {
		for (const [k, icon] of Object.entries(KANJI_ICONS)) {
			const name = /^i-solar-([a-z0-9-]+-bold-duotone)$/.exec(icon)?.[1];
			expect(name && solar.has(name), `${k}: ${icon}`).toBe(true);
		}
	});
});
