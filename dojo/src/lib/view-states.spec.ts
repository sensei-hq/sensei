// billing build (Design, phase A) — D87 remembered panels for the dōjō. The shell
// stores nothing; the dōjō keeps every view's panels in one map, keyed by view id,
// in localStorage `sensei.dojo.views` (CONTRACT §4). The first visit opens
// everything (an empty state — a missing key is open); later visits restore it.
// A failed or missing read is a first visit, never an error on the screen.
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

// ── test-local shape of the module the build adds; phase B imports it directly ──
type PanelState = Partial<Record<'info' | 'stats' | 'filters' | 'side', boolean>>;
type ViewStates = { panelsFor(viewId: string): PanelState; savePanels(viewId: string, next: PanelState): void; STORAGE_KEY: string };
const load = async () => (await import(/* @vite-ignore */ './view-states' + '.svelte')) as unknown as ViewStates;

describe('dōjō view states (D87)', () => {
	// An in-memory store, as glyphs.spec.ts does: Node's own experimental
	// localStorage global shadows jsdom's here and has no clear().
	let store: Map<string, string>;
	beforeEach(() => {
		store = new Map<string, string>();
		vi.stubGlobal('localStorage', {
			getItem: (k: string) => store.get(k) ?? null,
			setItem: (k: string, v: string) => void store.set(k, v)
		});
	});
	afterEach(() => vi.unstubAllGlobals());

	it.skip('pending: billing-build — the first visit of a view is everything open', async () => {
		const v = await load();
		expect(v.STORAGE_KEY).toBe('sensei.dojo.views');
		expect(v.panelsFor('dojo.org.billing')).toEqual({});
	});

	it.skip('pending: billing-build — a later visit restores what the user left, per view id', async () => {
		const v = await load();
		v.savePanels('dojo.org.billing', { info: false });
		expect(v.panelsFor('dojo.org.billing')).toEqual({ info: false });
		expect(v.panelsFor('dojo.org.members')).toEqual({});
		expect(JSON.parse(store.get('sensei.dojo.views')!)).toEqual({ 'dojo.org.billing': { info: false } });
	});

	it.skip('pending: billing-build — unreadable storage reads as a first visit', async () => {
		store.set('sensei.dojo.views', '{not json');
		const v = await load();
		expect(v.panelsFor('dojo.org.billing')).toEqual({});
	});

	// CONTRACT §4: "a failed or missing read means first visit" — a private window's
	// accessor throws; that is a first visit too, never an error.
	it.skip('pending: billing-build — a read that throws reads as a first visit', async () => {
		vi.stubGlobal('localStorage', {
			getItem: () => {
				throw new DOMException('denied', 'SecurityError');
			},
			setItem: () => {
				throw new DOMException('denied', 'SecurityError');
			}
		});
		const v = await load();
		expect(() => v.panelsFor('dojo.org.billing')).not.toThrow();
		expect(v.panelsFor('dojo.org.billing')).toEqual({});
	});
});
