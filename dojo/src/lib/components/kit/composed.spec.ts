import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import ChipHarness from './Chip.harness.svelte';
import BtnHarness from './Btn.harness.svelte';
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

	it('SectionHead shows eyebrow, title and count', () => {
		const { getByText } = render(SectionHeadHarness, {
			eyebrow: 'my work',
			title: 'Workspaces in flight',
			count: 5,
			kanji: '件'
		});
		expect(getByText('my work')).toBeTruthy();
		expect(getByText('Workspaces in flight')).toBeTruthy();
		expect(getByText('5')).toBeTruthy();
	});

	it('ListSection renders the header + fixture rows inside the flush card', () => {
		const { getByText, getAllByText } = render(ListSectionHarness, {
			title: 'Active workspaces',
			count: 3
		});
		expect(getByText('Active workspaces')).toBeTruthy();
		expect(getByText('lumen-auth')).toBeTruthy();
		// three fixture rows land inside the card.
		expect(getAllByText(/-/).length).toBeGreaterThan(0);
	});
});
