<script lang="ts">
	import { resolve } from '$app/paths';
	import type { DojoOrg } from '$lib/dojo-data';

	// Console top bar (mockup DojoTopBar): brand · org-switcher popover · knowledge
	// search · member count · avatar · log out. The switcher is a click-to-open,
	// keyboard-accessible popover: a pinned "Relay · you" personal home, one row
	// per membership, then "Your Dōjōs" + "＋ Create or join a Dōjō" (both → /orgs).
	//
	// This component is PRESENTATIONAL: switching an org emits `onSwitch(org)` and
	// the pinned home emits `onRelayHome()` — the layout wires those to the ONE
	// shared tenant-switch path (`enterOrg` → dojo_tenant cookie + goto /console),
	// so this bar never invents a second cookie/navigation path.
	//
	// `onMenu` opens the mobile nav drawer (the layout owns that state); on md:+ the
	// sidebar is always visible so the trigger is md:hidden. `navExpanded` reflects
	// the drawer's open state onto the trigger for assistive tech (aria-expanded).
	let {
		org,
		memberships = [],
		hasMembership = false,
		onSwitch,
		onRelayHome,
		onMenu,
		navExpanded = false
	}: {
		org: DojoOrg | undefined;
		memberships?: DojoOrg[];
		hasMembership?: boolean;
		onSwitch?: (org: DojoOrg) => void;
		onRelayHome?: () => void;
		onMenu?: () => void;
		navExpanded?: boolean;
	} = $props();

	// The label on the switcher trigger: the selected org's name, or "Relay · you"
	// when the user is solo (no membership / no tenant selected).
	const triggerLabel = $derived(hasMembership && org ? org.name : 'Relay · you');
	const triggerKanji = $derived(hasMembership && org ? org.kanji : '場');

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

	// Popover open state + the trigger element (so focus returns to it on close).
	// An attachment captures the element without `bind:this` (Svelte 5 idiom).
	let swOpen = $state(false);
	let triggerEl: HTMLButtonElement | undefined;
	function captureTrigger(node: HTMLButtonElement) {
		triggerEl = node;
	}

	function closePopover() {
		swOpen = false;
		triggerEl?.focus();
	}

	function switchTo(m: DojoOrg) {
		swOpen = false;
		onSwitch?.(m);
	}

	function goRelayHome() {
		swOpen = false;
		onRelayHome?.();
	}

	// The currently-selected org (highlighted with a check in the list).
	const currentUrl = $derived(org?.url);
	const orgsHref = resolve('/orgs');
</script>

<div
	class="border-paper-edge bg-paper flex h-[54px] flex-shrink-0 items-center gap-2 border-b px-3 md:gap-4 md:px-4"
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

	<!-- org switcher popover — pinned Relay·you + memberships + create/join. -->
	<div class="relative" style="margin-left: 6px">
		<button
			{@attach captureTrigger}
			type="button"
			aria-label="Switch organization"
			aria-haspopup="menu"
			aria-expanded={swOpen}
			onclick={() => (swOpen = !swOpen)}
			onkeydown={(e) => {
				if (e.key === 'Escape' && swOpen) {
					e.preventDefault();
					closePopover();
				}
			}}
			class="bg-paper-soft inline-flex cursor-pointer items-center gap-2 rounded-lg border {swOpen
				? 'border-accent'
				: 'border-paper-edge'}"
			style="padding: 4px 12px"
		>
			<span class="kanji text-accent text-sm">{triggerKanji}</span>
			<span class="text-ink text-sm">{triggerLabel}</span>
			{#if hasMembership && org}
				<span class="mono text-ink-faint text-xs uppercase" style="letter-spacing: 0.08em"
					>{org.kind}</span
				>
			{/if}
			<span class="text-ink-mute text-xs" aria-hidden="true">▾</span>
		</button>

		{#if swOpen}
			<!-- Backdrop: a click outside closes the popover (focus returns to the
			     trigger). Keyboard users close with Escape on the trigger. -->
			<button
				type="button"
				tabindex="-1"
				aria-hidden="true"
				class="fixed inset-0 z-40 cursor-default border-none bg-transparent"
				onclick={closePopover}
			></button>
			<div
				role="menu"
				aria-label="Organizations"
				class="bg-paper border-paper-edge absolute left-0 z-50 overflow-hidden rounded-lg border shadow-lg"
				style="top: calc(100% + 6px); width: 300px"
			>
				<!-- Pinned: the personal home, keyed by you across all Dōjōs. -->
				<button
					type="button"
					role="menuitem"
					onclick={goRelayHome}
					class="bg-accent-soft border-paper-edge flex w-full cursor-pointer items-center gap-3 border-b border-l-0 border-r-0 border-t-0 text-left"
					style="padding: 8px 12px"
				>
					<span class="kanji text-accent text-base text-center" style="width: 18px">場</span>
					<span class="min-w-0 flex-1">
						<span class="text-ink block text-sm font-medium">Relay · you</span>
						<span class="text-ink-mute block text-xs">all Dōjōs · no switching needed</span>
					</span>
				</button>

				{#if memberships.length > 0}
					<div style="max-height: 280px; overflow: auto; padding: 4px 0">
						{#each memberships as m (m.id)}
							{@const on = m.url === currentUrl}
							<button
								type="button"
								role="menuitem"
								onclick={() => switchTo(m)}
								class="flex w-full cursor-pointer items-center gap-3 border-none text-left {on
									? 'bg-paper-soft'
									: 'bg-transparent'}"
								style="padding: 8px 12px"
							>
								<span class="kanji text-accent text-sm text-center" style="width: 18px">{m.kanji}</span>
								<span class="min-w-0 flex-1">
									<span class="text-ink block truncate text-sm">{m.name}</span>
									<span class="mono text-ink-faint block text-xs">{m.role} · {m.kind}</span>
								</span>
								{#if on}
									<span class="text-accent text-sm" aria-hidden="true">✓</span>
								{/if}
							</button>
						{/each}
					</div>
				{/if}

				<!-- Manage all memberships / create-or-join — the org-picker route. -->
				<a
					href={orgsHref}
					role="menuitem"
					onclick={() => (swOpen = false)}
					class="border-paper-edge text-ink flex w-full items-center gap-3 border-b-0 border-l-0 border-r-0 border-t text-left no-underline"
					style="padding: 8px 12px"
				>
					<span class="kanji text-ink-mute text-sm text-center" style="width: 18px">群</span>
					<span class="min-w-0 flex-1">
						<span class="text-ink block text-sm">Your Dōjōs</span>
						<span class="text-ink-faint block text-xs">see &amp; manage all</span>
					</span>
					<span class="text-ink-faint text-sm" aria-hidden="true">→</span>
				</a>
				<a
					href={orgsHref}
					role="menuitem"
					onclick={() => (swOpen = false)}
					class="border-paper-edge text-ink-soft flex w-full items-center gap-2 border-b-0 border-l-0 border-r-0 border-t text-sm no-underline"
					style="padding: 12px"
				>
					<span class="text-accent" aria-hidden="true">＋</span>
					<span>Create or join a Dōjō</span>
				</a>
			</div>
		{/if}
	</div>

	<span class="flex-1"></span>

	<div
		class="bg-paper-soft border-paper-edge hidden items-center gap-2 rounded-lg border md:flex"
		style="padding: 4px 12px; width: 260px"
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
		style="padding: 4px 12px; background: transparent; cursor: pointer"
		title="Log out"
	>
		<span class="kanji text-ink-mute text-xs">出</span>
		<span class="hidden text-xs md:inline">Log out</span>
	</button>
</div>
