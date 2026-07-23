<script lang="ts">
	import { resolve } from '$app/paths';
	import { dojoBuild } from '$lib/version';

	// Left nav (mockup DojoNav): grouped destinations with a kanji glyph. Overview
	// + Triage (R9) and the admin console screens (R10: Members, Identities,
	// Policies, Health, Audit) are wired; the rest render as "soon"
	// (non-interactive) per the mockup's DOJO_BUILT gating. `active` is the current
	// section id.
	// `open`/`onClose` drive the mobile drawer (md:+ renders a static sidebar where
	// `open` is irrelevant). `onClose` fires from the backdrop and on navigation.
	let {
		active,
		tenantKey,
		open = false,
		onClose
	}: { active: string; tenantKey: string; open?: boolean; onClose?: () => void } = $props();

	// A wired destination's route id (`to`); absent → a "soon" placeholder. Each id
	// maps to its console sub-route below.
	type RouteId =
		| 'overview'
		| 'triage'
		| 'relay'
		| 'members'
		| 'identities'
		| 'policies'
		| 'health'
		| 'audit'
		| 'engagements'
		| 'incidents';

	interface NavItem {
		id: string;
		kanji: string;
		label: string;
		to?: RouteId;
		badge?: number;
	}
	interface NavGroup {
		group: string;
		items: NavItem[];
	}

	// Overview + Triage (R9) and the admin screens (R10) are wired; the rest stay
	// "soon". The href for a wired item comes from hrefFor(to).
	const groups = $derived<NavGroup[]>([
		{
			group: 'Govern',
			items: [
				{ id: 'overview', kanji: '全', label: 'Overview', to: 'overview' },
				{ id: 'triage', kanji: '門', label: 'Triage', to: 'triage' },
				{ id: 'relay', kanji: '継', label: 'Relay', to: 'relay' },
				{ id: 'knowledge', kanji: '蔵', label: 'Knowledge' }
			]
		},
		{
			group: 'Org',
			items: [
				{ id: 'members', kanji: '任', label: 'Members & roles', to: 'members' },
				{ id: 'identities', kanji: '鍵', label: 'Identities', to: 'identities' },
				{ id: 'policies', kanji: '規', label: 'Policies', to: 'policies' },
				{ id: 'scopes', kanji: '層', label: 'Scopes' }
			]
		},
		{
			group: 'Clients',
			items: [
				{ id: 'engagements', kanji: '客', label: 'Engagements', to: 'engagements' },
				{ id: 'incidents', kanji: '警', label: 'Incidents', to: 'incidents' }
			]
		},
		{
			group: 'Trust',
			items: [
				{ id: 'health', kanji: '観', label: 'Health', to: 'health' },
				{ id: 'audit', kanji: '録', label: 'Audit trail', to: 'audit' }
			]
		}
	]);

	// Route path for a wired destination. Overview is the console index; every
	// other wired id is a `/console/{id}` sub-route.
	function hrefFor(to: RouteId): string {
		switch (to) {
			case 'overview':
				return resolve('/(console)/console');
			case 'triage':
				return resolve('/(console)/console/triage');
			case 'relay':
				return resolve('/(console)/console/relay');
			case 'members':
				return resolve('/(console)/console/members');
			case 'identities':
				return resolve('/(console)/console/identities');
			case 'policies':
				return resolve('/(console)/console/policies');
			case 'health':
				return resolve('/(console)/console/health');
			case 'audit':
				return resolve('/(console)/console/audit');
			case 'engagements':
				return resolve('/(console)/console/engagements');
			case 'incidents':
				return resolve('/(console)/console/incidents');
		}
	}
</script>

{#if open}
	<button
		type="button"
		aria-label="Close navigation"
		onclick={() => onClose?.()}
		class="bg-ink fixed inset-0 z-40 cursor-pointer border-none opacity-50 md:hidden"
	></button>
{/if}
<aside
	id="console-nav"
	class="border-paper-edge bg-paper-soft fixed inset-y-0 left-0 z-50 flex w-[218px] flex-shrink-0 flex-col overflow-auto border-r px-3 py-4 transition-transform duration-200 md:static md:z-auto md:translate-x-0 md:transition-none {open
		? 'translate-x-0'
		: '-translate-x-full'}"
>
	{#each groups as grp (grp.group)}
		<div style="margin-bottom: 14px">
			<div
				class="text-ink-faint text-xs font-semibold uppercase"
				style="letter-spacing: 0.14em; padding: 0 8px; margin-bottom: 6px"
			>
				{grp.group}
			</div>
			<div class="flex flex-col gap-1">
				{#each grp.items as it (it.id)}
					{@const on = active === it.id}
					{#if it.to}
						<a
							href={hrefFor(it.to)}
							onclick={() => onClose?.()}
							aria-current={on ? 'page' : undefined}
							class="grid w-full items-center gap-2 rounded-lg text-sm no-underline {on
								? 'bg-paper border-paper-edge text-ink border'
								: 'text-ink-soft border border-transparent'}"
							style="grid-template-columns: auto 1fr auto; text-align: left; padding: 8px 8px"
						>
							<span
								class="kanji text-sm text-center {on ? 'text-accent' : 'text-ink-mute'}"
								style="width: 15px">{it.kanji}</span
							>
							<span>{it.label}</span>
							{#if it.badge != null}
								<span
									class="mono bg-accent text-on-primary rounded-full text-xs font-semibold"
									style="padding: 0 8px; line-height: 16px">{it.badge}</span
								>
							{:else}
								<span></span>
							{/if}
						</a>
					{:else}
						<div
							title="Designed in a later pass"
							class="text-ink-faint grid w-full items-center gap-2 rounded-lg border border-transparent text-sm"
							style="grid-template-columns: auto 1fr auto; text-align: left; padding: 8px 8px; opacity: 0.6"
						>
							<span class="kanji text-ink-mute text-sm text-center" style="width: 15px">{it.kanji}</span>
							<span>{it.label}</span>
							<span class="text-ink-faint text-xs" style="letter-spacing: 0.06em">soon</span>
						</div>
					{/if}
				{/each}
			</div>
		</div>
	{/each}

	<div class="flex-1"></div>
	<div
		class="border-paper-edge text-ink-mute grid w-full items-center gap-2 border-t text-sm"
		style="grid-template-columns: auto 1fr; text-align: left; padding: 12px 8px 8px; opacity: 0.6"
	>
		<span class="kanji text-ink-mute text-sm text-center" style="width: 15px">調</span>
		<span>Settings · SSO</span>
	</div>
	<!-- Build-time version stamp (src/lib/version.ts) so a live deploy is
	     verifiable at a glance; title carries the ISO build time. -->
	<div
		data-testid="dojo-version"
		class="text-ink-faint text-xs"
		style="padding: 0 8px 4px"
		title={dojoBuild.builtAt}
	>
		dojo v{dojoBuild.version} · {dojoBuild.gitSha}
	</div>
</aside>
