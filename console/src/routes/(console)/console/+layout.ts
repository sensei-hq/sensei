import type { LayoutLoad } from './$types';
import { tenantKeyFromUrl, orgForTenant } from '$lib/tenant';

// Resolve the selected tenant for every console screen from the URL (?tenant=…),
// falling back to the default when absent. The org record (chrome: name, kanji,
// role, members) comes from the R6 static org list. Runs on both server and
// client so the shell renders identically under SSR/prerender.
export const load: LayoutLoad = ({ url }) => {
	const tenantKey = tenantKeyFromUrl(url);
	return {
		tenantKey,
		org: orgForTenant(tenantKey)
	};
};
