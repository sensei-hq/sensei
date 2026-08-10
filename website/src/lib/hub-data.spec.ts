import { describe, it, expect } from 'vitest';
import {
	PRODUCTS,
	INCUBATING,
	NAV_LINKS,
	REPOS,
	productList,
	libraryList,
	featuredProduct
} from './hub-data';
import rokkitConfig from '../../rokkit.config.js';

describe('hub-data — portfolio model', () => {
	it('splits into 3 products and 4 libraries', () => {
		expect(productList().map((p) => p.id)).toEqual(['sensei', 'torii', 'seiki']);
		expect(libraryList().map((p) => p.id)).toEqual(['gateway', 'dbd', 'rokkit', 'kavach']);
	});

	it('every entry has kind product or library', () => {
		for (const p of PRODUCTS) expect(['product', 'library']).toContain(p.kind);
	});

	it('exactly one featured product, and it is Sensei', () => {
		expect(PRODUCTS.filter((p) => p.featured)).toHaveLength(1);
		const f = featuredProduct();
		expect(f?.id).toBe('sensei');
		expect(f?.kind).toBe('product');
		expect(f?.highlights?.length ?? 0).toBeGreaterThan(0);
	});

	it('libraries all declare a language; products do not need one', () => {
		for (const p of libraryList()) expect(p.lang, p.id).toBeTruthy();
	});

	it('ids are unique and the index restarts per kind', () => {
		const ids = PRODUCTS.map((p) => p.id);
		expect(new Set(ids).size).toBe(ids.length);
		expect(productList().map((p) => p.index)).toEqual(['01', '02', '03']);
		expect(libraryList().map((p) => p.index)).toEqual(['01', '02', '03', '04']);
	});

	it('every entry has the required non-empty copy', () => {
		const required = ['kanji', 'name', 'category', 'tagline', 'blurb', 'status', 'href'] as const;
		for (const p of PRODUCTS) {
			for (const k of required) expect(p[k], `${p.id}.${k}`).toBeTruthy();
			expect(p.meta.length, `${p.id}.meta`).toBeGreaterThan(0);
		}
	});

	it('hrefs are internal (/) or external (https://)', () => {
		for (const p of PRODUCTS) {
			expect(/^(\/|https:\/\/)/.test(p.href), `${p.id} href=${p.href}`).toBe(true);
		}
	});

	it('nav links include Products and Libraries', () => {
		const labels = NAV_LINKS.map(([, label]) => label);
		expect(labels).toContain('Products');
		expect(labels).toContain('Libraries');
	});

	it('open-source repos all point at github.com/sensei-hq', () => {
		for (const r of REPOS) expect(r.href, r.name).toMatch(/^https:\/\/github\.com\/sensei-hq\//);
	});

	// The invariant that keeps "add a product" a pure data change: every product,
	// library and incubating id must have a matching per-product accent token.
	it('every id has an accent token in rokkit.config overrides', () => {
		const overrides = (rokkitConfig as { overrides: Record<string, unknown> }).overrides;
		for (const p of PRODUCTS) expect(overrides[p.id], `accent token: ${p.id}`).toBeDefined();
		for (const p of INCUBATING) expect(overrides[p.id], `accent token: ${p.id}`).toBeDefined();
	});
});
