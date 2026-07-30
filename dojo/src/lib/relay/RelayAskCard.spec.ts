import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import RelayAskCard from './RelayAskCard.svelte';
import type { RelayAsk } from './types';

// Component render test: an ask in → the mockup AskCard out. Locks the verb label
// (Solar-icon kind, not kanji), the selectable options + freeform → onanswer wiring,
// and the answered verdict echo.
function ask(over: Partial<RelayAsk> = {}): RelayAsk {
	return {
		id: 'a1',
		action: 'approve',
		blocking: true,
		prompt: 'Run the staging migration?',
		context: 'touches an auth-boundary schema',
		options: ['Run it', 'Dry-run first', 'Skip the migration'],
		taskTitle: 'Staging migration',
		createdAt: new Date().toISOString(),
		...over
	};
}

afterEach(cleanup);

describe('RelayAskCard', () => {
	it('renders the verb label, prompt, context, holds-line and options', () => {
		const { getByText } = render(RelayAskCard, { ask: ask() });
		expect(getByText('approve')).toBeTruthy();
		expect(getByText('Run the staging migration?')).toBeTruthy();
		expect(getByText('touches an auth-boundary schema')).toBeTruthy();
		expect(getByText('Run it')).toBeTruthy();
		expect(getByText('Skip the migration')).toBeTruthy();
	});

	it('sends the picked option through onanswer', async () => {
		const onanswer = vi.fn();
		const { getByText } = render(RelayAskCard, { ask: ask(), onanswer });
		await fireEvent.click(getByText('Dry-run first'));
		await fireEvent.click(getByText('Send answer'));
		expect(onanswer).toHaveBeenCalledWith('a1', 'Dry-run first');
	});

	it('shows the answered verdict echo and drops the answer form', () => {
		const { getByText, queryByText } = render(RelayAskCard, { ask: ask(), verdict: 'Run it' });
		expect(getByText(/Run it/)).toBeTruthy();
		expect(queryByText('Send answer')).toBeNull();
	});
});
