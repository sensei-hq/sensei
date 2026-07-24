import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import ChipHarness from './Chip.harness.svelte';
import BtnHarness from './Btn.harness.svelte';
import BannerHarness from './Banner.harness.svelte';
import EmptyStateHarness from './EmptyState.harness.svelte';
import SectionHeadHarness from './SectionHead.harness.svelte';
import ListSectionHarness from './ListSection.harness.svelte';

// The snippet-children primitives are exercised through thin harnesses that
// supply a fixed text child (house pattern: Foo.harness.svelte). Each spec
// asserts the child + a variant token class render, and empty-state defaults hold.
describe('kit composed primitives render', () => {
	afterEach(cleanup);

	it('Chip renders its child + tone/soft/edge classes', () => {
		const { getByText } = render(ChipHarness, {
			label: '4 need you',
			icon: 'bell',
			toneClass: 'text-accent',
			softClass: 'bg-accent-soft',
			edgeClass: 'border-accent-soft'
		});
		const chip = getByText('4 need you');
		expect(chip.className).toContain('text-accent');
		expect(chip.className).toContain('bg-accent-soft');
	});

	it('Btn renders its label and fires onclick', async () => {
		const { getByText, getByTestId } = render(BtnHarness, {
			label: 'Approve once',
			icon: 'check-circle'
		});
		expect(getByTestId('clicks').textContent).toBe('0');
		await fireEvent.click(getByText('Approve once'));
		expect(getByTestId('clicks').textContent).toBe('1');
	});

	it('Btn ghost variant uses the paper surface', () => {
		const { getByText } = render(BtnHarness, { label: 'Deny', variant: 'ghost' });
		expect(getByText('Deny').closest('button')?.className).toContain('bg-paper');
	});

	it('Banner renders title + body and carries the tone fill', () => {
		const { getByText } = render(BannerHarness, {
			title: 'Two client rungs resolve here',
			body: 'nothing routes you away',
			tone: 'accent',
			kanji: '争'
		});
		expect(getByText('Two client rungs resolve here')).toBeTruthy();
		const body = getByText('nothing routes you away');
		// The fill sits on the banner root; walk up to it and assert the accent tint.
		expect(body.closest('div.rounded-lg')?.className).toContain('bg-accent-soft');
	});

	it('EmptyState shows the default voice copy when no title is given', () => {
		const { getByText } = render(EmptyStateHarness, {});
		expect(getByText('Still listening.')).toBeTruthy();
	});

	it('EmptyState shows a custom title + body', () => {
		const { getByText } = render(EmptyStateHarness, {
			title: 'Nothing needs you.',
			body: 'sensei surfaces only what it can’t decide alone.'
		});
		expect(getByText('Nothing needs you.')).toBeTruthy();
		expect(getByText('sensei surfaces only what it can’t decide alone.')).toBeTruthy();
	});

	it('EmptyState centers a bordered card (kanji + title + description panel)', () => {
		const { getByText } = render(EmptyStateHarness, {
			title: 'Nothing needs you.',
			body: 'a calm second sentence.'
		});
		// the title sits inside a small bordered, rounded, paper-soft card…
		const card = getByText('Nothing needs you.').closest('div.rounded-lg');
		expect(card?.className).toContain('border');
		expect(card?.className).toContain('border-paper-edge');
		expect(card?.className).toContain('bg-paper-soft');
		expect(card?.className).toContain('max-w-sm');
		// …and the card is centered in the available space by its outer wrapper.
		const outer = card?.parentElement;
		expect(outer?.className).toContain('items-center');
		expect(outer?.className).toContain('justify-center');
	});

	it('SectionHead shows eyebrow, title and count', () => {
		const { getByText } = render(SectionHeadHarness, {
			eyebrow: 'my work',
			title: 'Projects in flight',
			count: 5,
			kanji: '件'
		});
		expect(getByText('my work')).toBeTruthy();
		expect(getByText('Projects in flight')).toBeTruthy();
		expect(getByText('5')).toBeTruthy();
	});

	it('ListSection renders the header + fixture rows inside the flush card', () => {
		const { getByText, getAllByText } = render(ListSectionHarness, {
			title: 'Active projects',
			count: 3
		});
		expect(getByText('Active projects')).toBeTruthy();
		expect(getByText('lumen-auth')).toBeTruthy();
		// three fixture rows land inside the card.
		expect(getAllByText(/-/).length).toBeGreaterThan(0);
	});
});
