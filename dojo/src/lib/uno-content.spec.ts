import { describe, it, expect } from 'vitest';
import { contentInclude } from '../../../packages/glyphs/uno-content.js';

// #133: the vocabulary's icon classes live in packages/glyphs/index.js. UnoCSS's
// default scan skips plain .js, so the dōjō shipped no CSS for them and every
// vocabulary icon rendered blank (seen live on /you/dojos). The shared include
// list scans .js and .ts as well as components.
describe('the UnoCSS content include shared through packages/glyphs', () => {
	const scans = (path: string) => contentInclude.some((r) => r.test(path));
	it('scans the glyph vocabulary module', () => {
		expect(scans('/repo/packages/glyphs/index.js')).toBe(true);
	});
	it('still scans components and TypeScript', () => {
		expect(scans('/repo/dojo/src/lib/components/kit/Chip.svelte')).toBe(true);
		expect(scans('/repo/dojo/src/lib/nav.ts')).toBe(true);
	});
});
