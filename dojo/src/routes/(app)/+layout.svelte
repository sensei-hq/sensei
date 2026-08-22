<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { AppShell } from '$lib/components/kit';
	import {
		navGroupsFor,
		tabsFor,
		youHref,
		orgHref,
		sectionFromYouPath,
		sectionFromOrgPath
	} from '$lib/nav';
	import { toKitDojos, toKitOrg, orgBySlug, roleKey, kitMe } from '$lib/chrome';
	import { enterOrg } from '$lib/tenant';

	// The dojo shell (mockup DojoApp2 / DojoApp2Mobile) — the K2AppShell frame for
	// BOTH the personal zone (`/you`, NAV_YOU) and the role-scoped org zone
	// (`/org/[slug]`, navForOrg). Unlike the mockup's in-memory `useState` nav, the
	// shell is ROUTE-driven: the context, the active section, and the visible nav
	// groups are all derived from the current URL, and selecting a destination
	// navigates. The org switcher reuses the ONE shared tenant-switch path
	// (`enterOrg`) — no second cookie/goto path.
	let { data, children } = $props();

	// Context = personal ("you") unless the path is under /org/[slug].
	const path = $derived(page.url.pathname);
	const isOrg = $derived(path.startsWith('/org/'));
	const slug = $derived(isOrg ? path.split('/')[2] : null);

	// The role that scopes the org nav — resolved from the membership matching the
	// URL slug (falls back to the layout's chrome org, then developer rank).
	const kitDojos = $derived(toKitDojos(data.memberships));
	const activeOrg = $derived(slug ? orgBySlug(data.memberships, slug) : undefined);
	const role = $derived(
		activeOrg ? roleKey(activeOrg.role) : data.org ? roleKey(data.org.role) : 'developer'
	);

	const context = $derived<'you' | 'org'>(isOrg ? 'org' : 'you');
	const kitOrg = $derived(activeOrg ? toKitOrg(activeOrg) : data.org ? toKitOrg(data.org) : undefined);
	const nav = $derived(navGroupsFor(isOrg ? role : null));
	const tabs = $derived(tabsFor(isOrg ? role : null));
	const active = $derived(isOrg ? sectionFromOrgPath(path) : sectionFromYouPath(path));
	const me = $derived(kitMe(data.user));

	// Nav selection → route. Org sections route under the active slug; personal
	// sections route under /you.
	function onnav(id: string) {
		goto(isOrg && slug ? orgHref(slug, id) : youHref(id));
	}

	// The needs-you bell / mobile "Needs" tab both land on the section that owns
	// the band — the landing for personal, the org home for org.
	function onneeds() {
		goto(isOrg && slug ? orgHref(slug) : youHref());
	}

	// Switcher pick: "you" → the personal landing; an org slug → the shared
	// tenant switch (persist the dojo_tenant cookie, then land on /org/[slug]).
	function onpick(picked: string) {
		if (picked === 'you') {
			goto(youHref());
			return;
		}
		const org = orgBySlug(data.memberships, picked);
		if (!org) return;
		enterOrg(org, {
			setCookie: (cookie) => (document.cookie = cookie),
			navigate: (to) => goto(to),
			consoleHref: orgHref(picked)
		});
	}

	// The needs-you bell count is wired off live `/v1` data in a later chunk; the
	// landing owns its own needs band off fixtures this chunk, so the chrome bell
	// stays quiet (0) rather than showing a fabricated count.
	const needsCount = 0;
</script>

<!-- One K2AppShell for every width: TopBar + responsive NavPane drawer, plus a
     bottom TabBar below `md`. Rendering a second phone-only shell here would
     instantiate `children()` — the whole screen tree — a second time. -->
<div class="bg-paper h-screen w-full overflow-hidden">
	<AppShell
		{context}
		org={kitOrg}
		dojos={kitDojos}
		{me}
		{nav}
		{tabs}
		{active}
		{needsCount}
		{onpick}
		{onnav}
		{onneeds}
	>
		{@render children()}
	</AppShell>
</div>
