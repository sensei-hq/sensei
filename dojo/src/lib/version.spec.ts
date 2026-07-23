import { describe, it, expect } from 'vitest';
import { dojoBuild } from './version';

// The `define`-injected globals (__DOJO_VERSION__ etc.) are NOT set under vitest
// — vitest runs its own config (vitest.config.ts) without the vite/SvelteKit
// `define` step. version.ts guards each with `typeof … === 'undefined'`, so in
// this env the three values fall back to the string 'unknown'. We therefore
// assert the module's SHAPE (the 3 keys, all strings) rather than exact build
// values; the real replacement is verified at `bun run build` time.
describe('dojoBuild', () => {
	it('exposes exactly version, gitSha and builtAt', () => {
		expect(Object.keys(dojoBuild).sort()).toEqual(['builtAt', 'gitSha', 'version']);
	});

	it('every field is a string', () => {
		expect(typeof dojoBuild.version).toBe('string');
		expect(typeof dojoBuild.gitSha).toBe('string');
		expect(typeof dojoBuild.builtAt).toBe('string');
	});
});
