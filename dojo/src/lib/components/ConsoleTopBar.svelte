<script lang="ts">
	import { resolve } from '$app/paths';
	import type { DojoOrg } from '$lib/dojo-data';

	// Console top bar (mockup DojoTopBar): brand · org switcher · knowledge search ·
	// member count · avatar · log out. Org details come from the selected tenant's
	// org record (chrome only — the switcher is a static affordance in R9; the
	// live org-switch flow is /orgs).
	let { org }: { org: DojoOrg | undefined } = $props();

	const initials = $derived(
		(org?.name ?? 'Dōjō')
			.split(/\s+/)
			.slice(0, 2)
			.map((w) => w[0]?.toUpperCase() ?? '')
			.join('')
	);
</script>

<div
	class="border-paper-edge bg-paper flex flex-shrink-0 items-center gap-4 border-b"
	style="height: 54px; padding: 0 18px"
>
	<div class="flex items-baseline gap-2">
		<span class="kanji text-accent" style="font-size: 22px; line-height: 1">結</span>
		<span class="display text-lg" style="letter-spacing: -0.01em">Dōjō</span>
	</div>

	<!-- org switcher — the multi-membership model (→ /orgs to change) -->
	<a
		href={resolve('/orgs')}
		class="bg-paper-soft border-paper-edge inline-flex items-center gap-2 rounded-lg border no-underline"
		style="margin-left: 6px; padding: 6px 11px"
	>
		<span class="kanji text-accent text-sm">{org?.kanji ?? '道'}</span>
		<span class="text-ink text-sm">{org?.name ?? 'Select organization'}</span>
		{#if org}
			<span class="mono text-ink-faint text-xs uppercase" style="letter-spacing: 0.08em">{org.kind}</span>
		{/if}
		<span class="text-ink-mute text-xs">▾</span>
	</a>

	<span class="flex-1"></span>

	<div
		class="bg-paper-soft border-paper-edge flex items-center gap-2 rounded-lg border"
		style="padding: 6px 11px; width: 260px"
	>
		<span class="kanji text-ink-mute text-xs">探</span>
		<span class="text-ink-faint text-xs">search knowledge…</span>
	</div>

	{#if org}
		<span class="mono text-ink-mute text-xs">{org.members} members</span>
	{/if}

	<span
		class="bg-accent-soft text-accent flex items-center justify-center rounded-full text-xs font-semibold"
		style="width: 28px; height: 28px"
		aria-hidden="true">{initials}</span
	>

	<!-- Log out is handled by the kavach sentry handle (config route /logout); R9
	     leaves it a static affordance to match the /orgs "sign out" placeholder. -->
	<button
		type="button"
		class="border-paper-edge text-ink-soft inline-flex items-center gap-2 rounded-lg border"
		style="padding: 6px 11px; background: transparent; cursor: pointer"
		title="Log out"
	>
		<span class="kanji text-ink-mute text-xs">出</span>
		<span class="text-xs">Log out</span>
	</button>
</div>
