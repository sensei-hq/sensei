<script lang="ts">
	import { resolve } from '$app/paths';
	import { dojoBuild } from '$lib/version';
	import { pendingContributionCount } from '$lib/developer-view';
	import {
		buildNavGroups,
		topGroups,
		manageGroups,
		hrefForRoute,
		type RouteId
	} from '$lib/nav-items';

	// Left nav (mockup DojoRoleNav + RELAY_GROUP): the personal zone on top — a
	// "Relay · you" group (the away-from-keyboard Relay surface) and the "Me"
	// group (teams 群 · contributions 共 · downstream 贈) — then the management
	// groups (Govern · Org · Clients · Trust) below a divider, de-emphasized.
	// The IA lives in `nav-items.ts` so ordering/wiring is unit-tested; this
	// component is presentational. Every wired route stays reachable; role-aware
	// filtering is Chunk 6. `active` is the current section id. `open`/`onClose`
	// drive the mobile drawer (md:+ renders a static sidebar where `open` is
	// irrelevant). `onClose` fires from the backdrop and on navigation.
	let {
		active,
		open = false,
		onClose
	}: { active: string; open?: boolean; onClose?: () => void } = $props();

	// The pending-contributions badge is view-state; the pure nav structure takes
	// it as an input so `nav-items.ts` stays free of $lib/developer-view.
	const groups = $derived(
		buildNavGroups({ pendingContributions: pendingContributionCount() })
	);
	const top = $derived(topGroups(groups));
	const manage = $derived(manageGroups(groups));

	function hrefFor(to: RouteId): string {
		return hrefForRoute(to, resolve);
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
	{#snippet navGroup(grp: (typeof groups)[number])}
		<div class="mb-4 {grp.manage ? 'opacity-80' : ''}">
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
	{/snippet}

	<!-- Personal zone (Relay · you + Me) — always on top. -->
	{#each top as grp (grp.group)}
		{@render navGroup(grp)}
	{/each}

	<!-- Spacer + divider: the management groups sit below, de-emphasized. -->
	<div class="flex-1" style="min-height: 12px"></div>
	{#if manage.length > 0}
		<div
			data-testid="nav-manage-divider"
			class="border-paper-edge border-t"
			style="margin: 0 8px 12px"
		></div>
	{/if}
	{#each manage as grp (grp.group)}
		{@render navGroup(grp)}
	{/each}

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
