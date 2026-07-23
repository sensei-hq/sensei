// The single membership guard for every org-scoped console screen (DJ1).
//
// A membership-less user has no tenant (`tenantKey === null`, the no-tenant
// sentinel from `resolveTenantKey`). Their org-scoped screens — members,
// identities, policies, triage, engagements, incidents, health, audit, relay —
// must NOT fetch `/v1/t/{tenant}/…` against a fabricated org; they show a shared
// "join or create a Dōjō" empty state instead.
//
// Rather than copy-paste the same `if (!tenantKey) return …` branch into a dozen
// `+page.ts` loads, they all call `guardTenantScope`: given the resolved tenant
// key and an async fetcher (the real list call), it short-circuits to a
// `noMembership` result when there is no tenant, and otherwise runs the fetcher.
// Pure over its inputs (the fetcher is injected), so it's unit-testable without a
// live backend and SSR-safe.
//
// The `TenantKey` (`string | null`) is owned by `$lib/tenant`.
import type { TenantKey } from './tenant';

/**
 * The outcome of guarding an org-scoped load: `noMembership` is `true` when the
 * caller has no tenant (membership-less) and the fetch was skipped, `false`
 * otherwise. The `value` is the fetcher's result for a member, or the supplied
 * empty fallback for a membership-less caller.
 */
export interface GuardedScope<T> {
	value: T;
	noMembership: boolean;
}

/**
 * Guard an org-scoped fetch behind membership.
 *
 * @param tenantKey the resolved tenant key — `null` means membership-less.
 * @param empty     the value to return (unfetched) for a membership-less caller.
 * @param fetcher   the real list call, invoked ONLY when a tenant is present.
 *
 * When `tenantKey` is `null`/empty the fetcher is never called (no network,
 * mechanical proof that a membership-less landing makes zero `/v1/t/…` calls);
 * the result is `{ value: empty, noMembership: true }`. Otherwise the fetcher
 * runs against the tenant and the result is `{ value, noMembership: false }`.
 */
export async function guardTenantScope<T>(
	tenantKey: TenantKey,
	empty: T,
	fetcher: (tenantKey: string) => Promise<T>
): Promise<GuardedScope<T>> {
	if (!tenantKey) {
		return { value: empty, noMembership: true };
	}
	return { value: await fetcher(tenantKey), noMembership: false };
}

/**
 * Narrow a `TenantKey` to a concrete tenant string, or throw. Used at
 * member-only call sites (mutation handlers, detail loads) that are only ever
 * reached with a real tenant — a membership-less user never sees the surface
 * that fires them. The throw is a guard against a logic error, not an expected
 * runtime path: no fabricated tenant, no silent no-op.
 */
export function requireTenant(tenantKey: TenantKey): string {
	if (!tenantKey) {
		throw new Error('no tenant selected — this action requires a Dōjō membership');
	}
	return tenantKey;
}
