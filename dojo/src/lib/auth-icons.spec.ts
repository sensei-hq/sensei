import { describe, expect, it, vi } from 'vitest';
import kavach from '../../kavach.config.js';

// The preset itself does not load under vitest (an extensionless ESM import in
// @rokkit/unocss); the shortcuts and safelist under test are this app's own.
vi.mock('@rokkit/unocss', () => ({ presetRokkit: () => ({ name: 'rokkit-stub' }) }));
const { default: uno } = await import('../../uno.config.js');

// kavach's AuthProvider renders `i-auth-{provider}` for every configured sign-in
// option, from node_modules UnoCSS never scans. Without a shortcut AND a
// safelist entry the class generates nothing and the button shows no logo — the
// state a newly added provider (Microsoft, #308 R5c) would ship in unnoticed.
describe('sign-in provider icons', () => {
	const shortcuts = new Map((uno.shortcuts as [string, string][]).map(([k, v]) => [k, v]));
	const safelist = new Set(uno.safelist as string[]);

	it.each(kavach.providers.map((p) => p.name))('%s has a generated icon', (name) => {
		expect(shortcuts.get(`i-auth-${name}`)).toMatch(/^i-/);
		expect(safelist.has(`i-auth-${name}`)).toBe(true);
	});

	it('shows the Microsoft logo for Microsoft', () => {
		expect(shortcuts.get('i-auth-azure')).toBe('i-simple-icons-microsoft');
	});
});
