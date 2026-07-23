import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import ScrProjects from './ScrProjects.svelte';
import ScrRelayApprove from './ScrRelayApprove.svelte';
import ScrRelayChat from './ScrRelayChat.svelte';
import ScrConstitution from './ScrConstitution.svelte';
import { projects, gates, chat, me, stance, ladder } from '$lib/components/kit/fixtures';

// px → 4px-grid cleanup intent (chunk 3). The (dojo2) screens' inline
// `padding/gap` px were converted to grid utilities (`p-8`=32 · `gap-6`=24 ·
// `p-4`=16 · `gap-4`=16). These assert the conversion preserved the layout
// intent: the desktop screens keep the 32/24 rhythm, the responsive screens
// switch to the 16/16 phone rhythm, and no inline `padding:`/`gap:` px survives
// on the screen wrappers. Rendered as the shell renders them (no mobile prop =
// desktop).

/** The root wrapper element a screen renders into. */
function root(container: HTMLElement): HTMLElement {
	return container.firstElementChild as HTMLElement;
}

describe('dojo2 screen spacing — px converted to 4px-grid utilities', () => {
	afterEach(cleanup);

	it('desktop screens use p-8 / gap-6 (32 / 24) — not inline px', () => {
		const { container } = render(ScrRelayApprove, { props: { gates } });
		const el = root(container);
		expect(el.className).toContain('p-8');
		expect(el.className).toContain('gap-6');
		// no inline padding/gap px left on the wrapper.
		expect(el.getAttribute('style') ?? '').not.toMatch(/padding:|gap:/);
	});

	it('a governance screen (ScrConstitution) uses the same desktop rhythm', () => {
		const { container } = render(ScrConstitution, { props: { stance, ladder } });
		const el = root(container);
		expect(el.className).toContain('p-8');
		expect(el.className).toContain('gap-6');
	});

	it('responsive screens keep the desktop rhythm by default (p-8 / gap-6)', () => {
		const { container } = render(ScrProjects, { props: { projects } });
		const el = root(container);
		expect(el.className).toContain('p-8');
		expect(el.className).toContain('gap-6');
		expect(el.getAttribute('style') ?? '').not.toMatch(/padding:|gap:/);
	});

	it('responsive screens switch to the phone rhythm with mobile (p-4 / gap-4)', () => {
		const { container } = render(ScrProjects, { props: { projects, mobile: true } });
		const el = root(container);
		expect(el.className).toContain('p-4');
		expect(el.className).toContain('gap-4');
	});

	it('ScrRelayChat wrapper carries p-8 / gap-6 with no inline spacing px', () => {
		const { container } = render(ScrRelayChat, { props: { thread: chat, me } });
		const el = root(container);
		expect(el.className).toContain('p-8');
		expect(el.className).toContain('gap-6');
		expect(el.getAttribute('style') ?? '').not.toMatch(/padding:|gap:/);
	});
});
