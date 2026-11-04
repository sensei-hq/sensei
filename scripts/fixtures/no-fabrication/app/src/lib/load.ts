// Fixture: a file that fetches, so T3 applies to it.
import { api } from './api';

export async function loadRows() {
	const rows = await fetch('/api/rows').then((r) => r.json()).catch(() => []); // expect: T1
	const one = await api.get('/one').catch((e) => null); // expect: T1
	void fetch('/ping').catch(() => {}); // expect: T1
	const items = rows.items ?? []; // expect: T3
	const total = rows.total || 0; // expect: T3
	const ratio = rows.ratio ?? 0.5;
	// const hidden = rows.hidden ?? [];
	const url = 'https://example.com/a ?? [] not code';
	return { items, total, one, ratio, url };
}

export async function loadTickets() {
	try {
		return await api.tickets();
	} catch { // expect: T2
		return [];
	}
}

export async function loadFixture() {
	try {
		return await api.plans();
	} catch (e) { // expect: T2
		return plansFixture;
	}
}

export async function loadRecorded(state: { error: unknown; rows: unknown[] }) {
	try {
		state.rows = await api.rows();
	} catch (e) {
		state.error = e;
		state.rows = [];
	}
}

export async function loadSurfaced() {
	try {
		return await api.rows();
	} catch (e) {
		throw new Error(`rows: ${e}`);
	}
}
