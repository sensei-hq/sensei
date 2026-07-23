import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import LadderRung from './LadderRung.svelte';
import ConflictCard from './ConflictCard.svelte';
import RuleRowHarness from './RuleRow.harness.svelte';
import StanceDialHarness from './StanceDial.harness.svelte';
import { ladder, conflicts, stance } from './fixtures';

// Render smoke tests for the governance-plane domain components (ladder / rule /
// conflict / stance). Each mounts with a fixture, asserts the key content + a
// variant, and — where the component is interactive — that the right callback
// fires. House pattern: interactive components exercised through a *.harness.svelte.
describe('kit governance components render', () => {
	afterEach(cleanup);

	it('LadderRung shows the scope identity, rule count and lock chip', () => {
		const { getByText } = render(LadderRung, { rung: ladder[0] });
		expect(getByText('Company')).toBeTruthy();
		expect(getByText('Acme Corp')).toBeTruthy();
		expect(getByText('your employer · every project')).toBeTruthy();
		expect(getByText('4 rules')).toBeTruthy();
		// company rung has 3 hard rules → a "3 locked" chip.
		expect(getByText('3 locked')).toBeTruthy();
	});

	it('LadderRung renders its rules when showRules and fires onSelect', async () => {
		let selected = '';
		const { getByText } = render(LadderRung, {
			rung: ladder[0],
			onSelect: (id: string) => (selected = id)
		});
		// a rule from the company rung is visible.
		expect(getByText('Never log tokens or PII, even at debug level')).toBeTruthy();
		await fireEvent.click(getByText('Acme Corp'));
		expect(selected).toBe('company');
	});

	it('LadderRung with showRules=false hides the rule list', () => {
		const { queryByText } = render(LadderRung, { rung: ladder[0], showRules: false });
		expect(queryByText('Never log tokens or PII, even at debug level')).toBeNull();
	});

	it('LadderRung tints an accent (client) rung', () => {
		const { getByText } = render(LadderRung, { rung: ladder[1] });
		expect(getByText('Client').className).toContain('text-accent');
	});

	it('RuleRow shows the rule text and the ★ marker for hard rules', () => {
		const { getByText } = render(RuleRowHarness, {});
		expect(getByText('Never log tokens or PII, even at debug level')).toBeTruthy();
		expect(getByText('non-negotiable')).toBeTruthy();
	});

	it('RuleRow shows the level chip when showLevel', () => {
		const { getByText } = render(RuleRowHarness, { showLevel: true });
		expect(getByText('Company')).toBeTruthy();
	});

	it('RuleRow fires onToggle when the include checkbox is clicked', async () => {
		const { getByLabelText, getByTestId } = render(RuleRowHarness, {
			withToggle: true,
			included: true
		});
		expect(getByTestId('toggles').textContent).toBe('0');
		await fireEvent.click(getByLabelText('Exclude rule'));
		expect(getByTestId('toggles').textContent).toBe('1');
	});

	it('RuleRow fires onJump on the level jump button, and onEdit on the pencil', async () => {
		const { getByTitle, getByLabelText, getByTestId } = render(RuleRowHarness, {
			showLevel: true,
			withJump: true,
			withEdit: true
		});
		await fireEvent.click(getByTitle('Jump to Company'));
		expect(getByTestId('jumps').textContent).toBe('1');
		await fireEvent.click(getByLabelText('Edit rule'));
		expect(getByTestId('edits').textContent).toBe('1');
	});

	it('ConflictCard shows topic, both sides, why and the settled/locked chip', () => {
		const { getByText } = render(ConflictCard, { conflict: conflicts[0] });
		expect(getByText('retry behaviour on money-moving calls')).toBeTruthy();
		expect(getByText('retry freely on transient failure')).toBeTruthy();
		expect(getByText('idempotency key required before retry')).toBeTruthy();
		expect(getByText(/More specific scope refines the broader one/)).toBeTruthy();
		// cf1 is not locked → "settled".
		expect(getByText('settled')).toBeTruthy();
	});

	it('ConflictCard shows the locked chip when a ★ decided it', () => {
		const { getByText } = render(ConflictCard, { conflict: conflicts[1] });
		expect(getByText('locked')).toBeTruthy();
	});

	it('StanceDial shows the label, caption and current level', () => {
		const { getByText } = render(StanceDialHarness, { dial: stance[0] });
		expect(getByText('autonomy')).toBeTruthy();
		expect(getByText('how far a session runs before it asks')).toBeTruthy();
		// value 1 → "ask on guarded" (the current level readout + the min-label share text,
		// so at least one node carries it).
		expect(getByText('run freely')).toBeTruthy();
	});

	it('StanceDial forwards onChange when a level dot is picked', async () => {
		const { getByLabelText, getByTestId } = render(StanceDialHarness, { dial: stance[0] });
		await fireEvent.click(getByLabelText('run freely'));
		expect(getByTestId('last-id').textContent).toBe('autonomy');
		expect(getByTestId('last-value').textContent).toBe('3');
	});
});
