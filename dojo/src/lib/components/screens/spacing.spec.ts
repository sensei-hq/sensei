import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import ScrProjects from './ScrProjects.svelte';
import ScrConstitution from './ScrConstitution.svelte';
import { projects, stance, ladder } from '$lib/components/kit/fixtures';

// Screen spacing is mobile-first (§1.7): the phone rhythm (`p-4`=16 · `gap-4`=16)
// is the base, and the desktop rhythm (`p-8`=32 · `gap-6`=24) sits behind `md:`.
// Before this, screens either hard-coded the desktop rhythm or branched on a
// `mobile` prop that nothing in the app ever set — so every screen rendered
// desktop spacing at every width.
//
// Asserted with word boundaries rather than `toContain`: `toContain('p-8')` also
// matches the `p-8` inside `md:p-8`, so it passed before *and* after the
// conversion and proved nothing either way.

/** The root wrapper element a screen renders into. */
function root(container: HTMLElement): HTMLElement {
	return container.firstElementChild as HTMLElement;
}

const SCREENS = [
	['ScrConstitution', () => render(ScrConstitution, { props: { stance, ladder } })],
	['ScrProjects', () => render(ScrProjects, { props: { projects } })]
] as const;

describe('dojo screen spacing — mobile-first 4px-grid utilities', () => {
	afterEach(cleanup);

	it.each(SCREENS)('%s bases the phone rhythm and steps up at md', (_name, mount) => {
		const cls = root(mount().container).className;
		expect(cls).toMatch(/\bp-4\b/);
		expect(cls).toMatch(/\bgap-4\b/);
		expect(cls).toMatch(/\bmd:p-8\b/);
		expect(cls).toMatch(/\bmd:gap-6\b/);
	});

	it.each(SCREENS)('%s has no unprefixed desktop spacing', (_name, mount) => {
		const cls = root(mount().container).className;
		// An unprefixed `p-8`/`gap-6` would apply at phone widths too — the exact
		// defect this conversion removed. The lookbehind lets `md:p-8` through.
		expect(cls).not.toMatch(/(?<!:)\bp-8\b/);
		expect(cls).not.toMatch(/(?<!:)\bgap-6\b/);
	});

	it.each(SCREENS)('%s keeps spacing out of inline styles', (_name, mount) => {
		const style = root(mount().container).getAttribute('style') ?? '';
		expect(style).not.toMatch(/padding:|gap:/);
	});
});

// There is no "renders the phone rhythm when `mobile` is set" case any more: the
// prop is gone from every screen, so the type checker — not a runtime assertion —
// is what stops the branch coming back. The unprefixed-spacing case above is the
// guard that matters, since that is the defect the prop was masking.
