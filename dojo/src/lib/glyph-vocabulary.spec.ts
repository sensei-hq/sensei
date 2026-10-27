import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'fs';
import { join } from 'path';
import { KANJI_ICONS } from '@sensei/glyphs';

// #133: the dōjō reads the same kanji→icon table as the desktop app
// (packages/glyphs). Every kanji the dōjō renders must be in it, every icon must
// be an installed Solar Bold Duotone class, and no mark opts out of icon mode:
// one rule across the app and the dōjō (user, 2026-10-27).
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
				if (!KANJI_ICONS[m[1]]) missing.set(m[1], file.replace('src/', ''));
			}
		}
		expect(Object.fromEntries(missing)).toEqual({});
	});

	it('lets no mark opt out of icon mode', () => {
		const opted = sources('src').filter((f) => /icon=\{null\}/.test(readFileSync(f, 'utf8')));
		expect(opted).toEqual([]);
	});

	it('maps every kanji to an installed Solar Bold Duotone icon', () => {
		for (const [k, icon] of Object.entries(KANJI_ICONS)) {
			const name = /^i-solar-([a-z0-9-]+-bold-duotone)$/.exec(icon)?.[1];
			expect(name && solar.has(name), `${k}: ${icon}`).toBe(true);
		}
	});
});
