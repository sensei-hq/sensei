<script lang="ts">
	import OrgSwitcher from './OrgSwitcher.svelte';
	import RoleTag from './RoleTag.svelte';
	import Icon from './Icon.svelte';
	import KanjiToken from './KanjiToken.svelte';
	import { createLogout } from './logout.svelte';
	import { getInitials } from './initials';
	import type { KitOrg, KitDojo, KitMe } from './types';

	// Top bar (kit K2TopBar): brand · org switcher · search affordance · needs-you
	// bell · avatar. Shared by both contexts. In an org an accent top-rule + a
	// route chip + role tag carry the "you've stepped into a managed place" signal.
	// The search field is a static affordance here (the live palette lands with the
	// screens); the mobile hamburger is emitted via `onmenu` (md:hidden trigger).
	let {
		context = 'you',
		org,
		dojos = [],
		me,
		needsCount = 0,
		navExpanded = false,
		onpick,
		onneeds,
		onmenu
	}: {
		context?: 'you' | 'org';
		org?: KitOrg;
		dojos?: KitDojo[];
		me?: KitMe;
		needsCount?: number;
		navExpanded?: boolean;
		onpick?: (slug: string) => void;
		onneeds?: () => void;
		onmenu?: () => void;
	} = $props();

	const isOrg = $derived(context === 'org');

	// Avatar as initials (the kit's window.Avatar) — two-letter monogram, matching
	// the shipped ConsoleTopBar treatment. Shared: kit/initials.ts.
	const initials = $derived(getInitials(me?.name));

	// The avatar IS the account/logout control (mockup has no separate Log out
	// button). Shared action, DRY with the mobile LogoutButton.
	const logout = createLogout();
</script>

<div
	class="border-paper-edge bg-paper flex flex-shrink-0 items-center gap-2 border-b px-3 md:gap-3 md:px-4 {isOrg
		? 'border-t-accent border-t-2'
		: ''}"
	style="height: 56px"
>
	<button
		type="button"
		onclick={() => onmenu?.()}
		aria-label="Open navigation"
		aria-controls="kit-nav"
		aria-expanded={navExpanded}
		class="text-ink-soft hover:text-ink -ml-1 flex h-[34px] w-[34px] flex-shrink-0 cursor-pointer items-center justify-center rounded-lg bg-transparent md:hidden"
	>
		<span aria-hidden="true" class="text-lg leading-none">☰</span>
	</button>

	<div class="flex flex-shrink-0 items-baseline gap-2">
		<span class="kanji text-accent" style="font-size: 20px; line-height: 1">結</span>
		<span class="display text-lg" style="letter-spacing: -0.01em">Dōjō</span>
	</div>

	<OrgSwitcher {context} {org} {dojos} {onpick} />

	{#if isOrg && org}
		<span
			class="mono text-ink-mute bg-paper-soft border-paper-edge hidden whitespace-nowrap rounded-full border text-xs md:inline"
			style="padding: 3px 10px">{org.route}</span
		>
		<span class="hidden md:inline"><RoleTag role={org.role} /></span>
	{/if}

	<span class="flex-1"></span>

	<div
		class="bg-paper-soft border-paper-edge hidden items-center gap-2 rounded border md:flex"
		style="padding: 0 12px; width: 240px; height: 34px"
	>
		<KanjiToken char="探" size="sm" toneClass="text-ink-mute" />
		<span class="text-ink-faint truncate text-sm">search…</span>
	</div>

	<button
		type="button"
		onclick={() => onneeds?.()}
		title={needsCount ? `${needsCount} need you` : 'Nothing needs you'}
		class="relative flex h-[34px] w-[34px] flex-shrink-0 items-center justify-center rounded-full border {needsCount
			? 'bg-accent-soft border-accent-soft'
			: 'border-paper-edge bg-transparent'}"
	>
		<Icon name="bell" size={17} toneClass={needsCount ? 'text-accent' : 'text-ink-mute'} />
		{#if needsCount > 0}
			<span
				class="mono bg-accent text-on-primary absolute text-xs font-semibold"
				style="top: -5px; right: -5px; min-width: 16px; height: 16px; padding: 0 4px; border-radius: 8px; line-height: 16px; text-align: center"
				>{needsCount}</span
			>
		{/if}
	</button>

	<!-- Avatar = the account/logout control (mockup: no separate Log out button). -->
	<button
		type="button"
		onclick={() => logout.run()}
		disabled={logout.busy}
		title="Log out"
		aria-label="Log out"
		class="bg-accent-soft text-accent flex flex-shrink-0 cursor-pointer items-center justify-center rounded-full border-none text-xs font-semibold"
		style="width: 30px; height: 30px"
	>
		{initials}
	</button>
</div>
