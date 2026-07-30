<script lang="ts">
	import { Toggle } from '@rokkit/ui';
	import { getInitials } from './initials';
	import { createLogout } from './logout.svelte';
	import { colorMode, setColorMode } from '$lib/theme.svelte';
	import type { ColorMode } from '@rokkit/app';
	import type { KitMe } from './types';

	// The avatar account menu — the top bar's account entry point. Clicking the avatar
	// opens a small popover (same trigger + backdrop pattern as OrgSwitcher; rokkit's
	// Menu/Dropdown render their own trigger, so they can't wrap an avatar) with the
	// theme mode switch (rokkit Toggle → system/light/dark, wired to the shared
	// colorMode manager) and Log out (shared createLogout).
	let { me }: { me?: KitMe } = $props();

	let open = $state(false);
	const initials = $derived(getInitials(me?.name));
	const logout = createLogout();

	const MODES = [
		{ label: 'System', value: 'system' },
		{ label: 'Light', value: 'light' },
		{ label: 'Dark', value: 'dark' }
	];

	function doLogout() {
		open = false;
		logout.run();
	}
</script>

<div class="relative">
	<button
		type="button"
		aria-haspopup="menu"
		aria-expanded={open}
		onclick={() => (open = !open)}
		onkeydown={(e) => {
			if (e.key === 'Escape' && open) {
				e.preventDefault();
				open = false;
			}
		}}
		title={me?.name ?? 'Account'}
		aria-label="Account menu"
		class="bg-accent-soft text-accent flex flex-shrink-0 cursor-pointer items-center justify-center rounded-full border-none text-xs font-semibold"
		style="width: 30px; height: 30px"
	>
		{initials}
	</button>

	{#if open}
		<button
			type="button"
			tabindex="-1"
			aria-hidden="true"
			class="fixed inset-0 z-40 cursor-default border-none bg-transparent"
			onclick={() => (open = false)}
		></button>
		<div
			role="menu"
			aria-label="Account"
			class="bg-paper border-paper-edge absolute right-0 z-50 overflow-hidden rounded-lg border shadow-lg"
			style="top: calc(100% + 6px); width: 240px"
		>
			{#if me?.name}
				<div class="border-paper-edge border-b px-4 py-3">
					<div class="text-ink truncate text-sm font-medium">{me.name}</div>
				</div>
			{/if}

			<div class="flex flex-col gap-2 px-4 py-3">
				<span class="text-ink-mute text-xs font-semibold uppercase" style="letter-spacing: 0.18em">Theme</span>
				<Toggle
					options={MODES}
					value={colorMode.mode}
					onchange={(v) => setColorMode(v as ColorMode)}
					size="sm"
					label="Theme"
				/>
			</div>

			<button
				type="button"
				role="menuitem"
				onclick={doLogout}
				disabled={logout.busy}
				class="border-paper-edge text-ink-soft hover:text-ink flex w-full cursor-pointer items-center gap-2 border-t bg-transparent px-4 py-3 text-left text-sm"
			>
				<span class="kanji text-ink-mute text-xs" aria-hidden="true">出</span> Log out
			</button>
		</div>
	{/if}
</div>
