import { describe, expect, it, vi } from 'vitest';
import { guardTenantScope, requireTenant } from '$lib/org-guard';

describe('guardTenantScope (the single DJ1 membership guard)', () => {
	it('skips the fetch and returns the empty fallback + noMembership when there is no tenant', async () => {
		const fetcher = vi.fn(async () => ['live']);
		const result = await guardTenantScope<string[]>(null, [], fetcher);
		expect(result).toEqual({ value: [], noMembership: true });
		// The proof that no /v1/t/{tenant}/… call is made for a membership-less user.
		expect(fetcher).not.toHaveBeenCalled();
	});

	it('treats an empty-string tenant key as no tenant (defensive)', async () => {
		const fetcher = vi.fn(async () => ['live']);
		const result = await guardTenantScope<string[]>('', [], fetcher);
		expect(result.noMembership).toBe(true);
		expect(fetcher).not.toHaveBeenCalled();
	});

	it('runs the fetcher against the tenant for a member and reports noMembership false', async () => {
		const fetcher = vi.fn(async (tk: string) => [`rows-for-${tk}`]);
		const result = await guardTenantScope<string[]>('github/globex', [], fetcher);
		expect(fetcher).toHaveBeenCalledWith('github/globex');
		expect(result).toEqual({ value: ['rows-for-github/globex'], noMembership: false });
	});

	it('propagates a fetcher rejection (the page load owns the try/catch → banner)', async () => {
		const fetcher = vi.fn(async () => {
			throw new Error('could not reach the dojo service');
		});
		await expect(guardTenantScope<string[]>('github/globex', [], fetcher)).rejects.toThrow(
			'could not reach the dojo service'
		);
	});

	it('supports a non-array empty fallback (e.g. a null rollup)', async () => {
		const fetcher = vi.fn(async () => ({ ok: true }));
		const result = await guardTenantScope<{ ok: boolean } | null>(null, null, fetcher);
		expect(result).toEqual({ value: null, noMembership: true });
		expect(fetcher).not.toHaveBeenCalled();
	});
});

describe('requireTenant (narrow-or-throw for member-only call sites)', () => {
	it('returns the tenant string when present', () => {
		expect(requireTenant('github/globex')).toBe('github/globex');
	});

	it('throws when there is no tenant (a logic error, never an expected path)', () => {
		expect(() => requireTenant(null)).toThrow(/requires a Dōjō membership/);
		expect(() => requireTenant('')).toThrow(/requires a Dōjō membership/);
	});
});
