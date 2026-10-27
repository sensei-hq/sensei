import { KIND_ICONS } from '@sensei/glyphs';
import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import KanjiToken from './KanjiToken.svelte';
import { glyphs } from '$lib/glyphs.svelte';

// #133: a dōjō mark follows the glyph choice — the shared vocabulary's icon in
// icon mode, the kanji in kanji mode; an identity mark (icon={null}) stays
// kanji, and its own classes carry over in both modes.
describe('KanjiToken and the glyph choice', () => {
	afterEach(() => {
		cleanup();
		glyphs.mode = 'kanji';
	});

	it('renders the vocabulary icon in icon mode', () => {
		glyphs.mode = 'icon';
		const { container } = render(KanjiToken, { char: '検', class: 'text-warning' });
		const el = container.querySelector('[data-component="glyph-icon"]') as HTMLElement;
		expect(el.className).toContain('i-solar-magnifer-bug-bold-duotone');
		expect(el.className).toContain('text-warning');
		expect(container.querySelector('.kanji')).toBeNull();
	});

	it('renders the kanji in kanji mode', () => {
		glyphs.mode = 'kanji';
		const { container } = render(KanjiToken, { char: '検' });
		expect(container.querySelector('.kanji')?.textContent).toBe('検');
	});

	// One rule (#133, 2026-10-27): the brand mark follows the mode too, and a
	// kanji that is data shows its kind's icon.
	it('shows the brand mark and a dōjō’s own kanji as icons in icon mode', () => {
		glyphs.mode = 'icon';
		const brand = render(KanjiToken, { char: '結' }).container;
		expect(brand.querySelector('[data-component="glyph-icon"]')?.className).toContain('i-solar-link-circle-bold-duotone');
		const own = render(KanjiToken, { char: '鬱', icon: KIND_ICONS.dojo }).container;
		expect(own.querySelector('.kanji')).toBeNull();
		expect(own.querySelector('[data-component="glyph-icon"]')?.className).toContain(KIND_ICONS.dojo);
	});
});
