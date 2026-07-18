<script lang="ts">
	import { resolve } from '$app/paths';
	import type { DojoOrg } from '$lib/dojo-data';

	// Console top bar (mockup DojoTopBar): brand · org switcher · knowledge search ·
	// member count · avatar · log out. Org details come from the selected tenant's
	// org record (chrome only — the switcher is a static affordance in R9; the
	// live org-switch flow is /orgs).
	// `onMenu` opens the mobile nav drawer (the layout owns the state); on md:+ the
	// sidebar is always visible so the trigger is md:hidden. `navExpanded` reflects the
	// drawer's open state onto the trigger for assistive tech (aria-expanded).
	let {
		org,
		onMenu,
		navExpanded = false
	}: { org: DojoOrg | undefined; onMenu?: () => void; navExpanded?: boolean } = $props();

	const initials = $derived(
		(org?.name ?? 'Dōjō')
			// Drop parenthetical qualifiers e.g. "Jerry (personal)" → "Jerry".
			.replace(/\([^)]*\)/g, ' ')
			.split(/\s+/)
			// Keep only tokens whose first character is alphanumeric, so stray
			// punctuation (e.g. a leftover "(") never becomes an initial.
			.filter((w) => /^[\p{L}\p{N}]/u.test(w))
			.slice(0, 2)
			.map((w) => w[0].toUpperCase())
			.join('')
	);
</script>

<div
	class="border-paper-edge bg-paper flex h-[54px] flex-shrink-0 items-center gap-2 border-b px-3 md:gap-4 md:px-[18px]"
>
	<button
		type="button"
		onclick={() => onMenu?.()}
		aria-label="Open navigation"
		aria-controls="console-nav"
		aria-expanded={navExpanded}
		class="text-ink-soft hover:text-ink -ml-1 flex h-[34px] w-[34px] cursor-pointer items-center justify-center rounded-lg bg-transparent md:hidden"
	>
		<span aria-hidden="true" class="text-lg leading-none">☰</span>
	</button>

	<div class="flex items-baseline gap-2">
		<span class="kanji text-accent text-xl" style="line-height: 1">結</span>
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
		class="bg-paper-soft border-paper-edge hidden items-center gap-2 rounded-lg border md:flex"
		style="padding: 6px 11px; width: 260px"
	>
		<span class="kanji text-ink-mute text-xs">探</span>
		<span class="text-ink-faint text-xs">search knowledge…</span>
	</div>

	{#if org}
		<span class="mono text-ink-mute hidden text-xs md:inline">{org.members} members</span>
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
		<span class="hidden text-xs md:inline">Log out</span>
	</button>
</div>
