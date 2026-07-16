import { render, cleanup, fireEvent, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import RelayGateCard from './RelayGateCard.svelte';
import type { RelayGate } from '$lib/relay-data';

// RelayGateCard render + reply tests: the "needs you" gate card shows the gate's
// prompt (or a friendly fallback), links to its run, and offers the right
// affordance by kind (approval → Approve/Decline, decision → one button per option,
// else → free-text). Replying posts through the global fetch (relayToGate uses
// opts.fetch ?? fetch) with the note folded in and fires onReplied on success.

const ZERO_UUID = '00000000-0000-0000-0000-000000000000';

function gate(overrides: Partial<RelayGate> = {}): RelayGate {
	return {
		id: 'g1',
		seq: 1,
		run_id: 'run-1',
		run_title: 'Round-trip',
		segment_id: null,
		kind: 'approval',
		payload: { prompt: 'Run the prod migration?' },
		created_at: new Date().toISOString(),
		...overrides
	};
}

function jsonResponse(body: unknown, status = 200): Response {
	return new Response(JSON.stringify(body), {
		status,
		headers: { 'content-type': 'application/json' }
	});
}

function stubFetch(impl?: (url: string, init?: RequestInit) => Response) {
	const fn = vi.fn((url: string, init?: RequestInit) =>
		Promise.resolve(impl ? impl(url, init) : jsonResponse({ id: 'i1', seq: 1 }))
	);
	vi.stubGlobal('fetch', fn);
	return fn;
}

describe('RelayGateCard', () => {
	afterEach(() => {
		cleanup();
		vi.unstubAllGlobals();
	});

	it('shows the prompt and links to the run', () => {
		stubFetch();
		const { getByText } = render(RelayGateCard, {
			gate: gate(),
			tenantKey: 'personal/jerry',
			accessToken: 'JWT'
		});
		expect(getByText('Run the prod migration?')).toBeTruthy();
		const link = getByText('Round-trip').closest('a');
		expect(link?.getAttribute('href')).toBe('/console/relay/run-1');
	});

	it('falls back to a friendly prompt and drops the run link for the all-zeros uuid', () => {
		stubFetch();
		const { getByText, queryByText } = render(RelayGateCard, {
			gate: gate({ payload: {}, run_id: ZERO_UUID, run_title: null }),
			tenantKey: 'personal/jerry',
			accessToken: null
		});
		expect(getByText('The run needs a decision')).toBeTruthy();
		expect(queryByText('Round-trip')).toBeNull();
	});

	it('approval posts { verdict: "approve" } with the note and fires onReplied', async () => {
		let sent: { inbox_id?: string; reply?: Record<string, unknown> } = {};
		const fetchFn = stubFetch((_url, init) => {
			sent = JSON.parse(String(init?.body));
			return jsonResponse({ id: 'i1', seq: 1 });
		});
		const onReplied = vi.fn();
		const { getByText, getByPlaceholderText } = render(RelayGateCard, {
			gate: gate(),
			tenantKey: 'personal/jerry',
			accessToken: 'JWT',
			onReplied
		});
		await fireEvent.input(getByPlaceholderText('Add a note (optional)…'), {
			target: { value: 'go for it' }
		});
		await fireEvent.click(getByText('Approve'));
		await waitFor(() => expect(onReplied).toHaveBeenCalledOnce());
		expect(fetchFn).toHaveBeenCalled();
		expect(sent.inbox_id).toBe('g1');
		expect(sent.reply).toEqual({ verdict: 'approve', note: 'go for it' });
	});

	it('decision renders one button per option and posts { choice }', async () => {
		let sent: { reply?: Record<string, unknown> } = {};
		stubFetch((_url, init) => {
			sent = JSON.parse(String(init?.body));
			return jsonResponse({ id: 'i1', seq: 1 });
		});
		const { getByText } = render(RelayGateCard, {
			gate: gate({ kind: 'decision', payload: { prompt: 'Which strategy?', options: ['JWT', 'Sessions'] } }),
			tenantKey: 'personal/jerry',
			accessToken: 'JWT'
		});
		await fireEvent.click(getByText('Sessions'));
		await waitFor(() => expect(sent.reply).toEqual({ choice: 'Sessions' }));
	});

	it('free-text posts { text } for a non-approval, non-decision kind', async () => {
		let sent: { reply?: Record<string, unknown> } = {};
		stubFetch((_url, init) => {
			sent = JSON.parse(String(init?.body));
			return jsonResponse({ id: 'i1', seq: 1 });
		});
		const { getByText, getByPlaceholderText } = render(RelayGateCard, {
			gate: gate({ kind: 'chat', payload: { prompt: 'What next?' } }),
			tenantKey: 'personal/jerry',
			accessToken: 'JWT'
		});
		await fireEvent.input(getByPlaceholderText('Type your answer…'), {
			target: { value: 'ship it' }
		});
		await fireEvent.click(getByText('Send'));
		await waitFor(() => expect(sent.reply).toEqual({ text: 'ship it' }));
	});

	it('surfaces a DojoApiError as an inline reply error and does not fire onReplied', async () => {
		stubFetch(() => jsonResponse({ error: 'forbidden' }, 403));
		const onReplied = vi.fn();
		const { getByText, findByText } = render(RelayGateCard, {
			gate: gate(),
			tenantKey: 'personal/jerry',
			accessToken: 'JWT',
			onReplied
		});
		await fireEvent.click(getByText('Decline'));
		expect(await findByText(/Reply not sent\./)).toBeTruthy();
		expect(onReplied).not.toHaveBeenCalled();
	});
});
