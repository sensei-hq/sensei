<script lang="ts">
	import KanjiToken from '$lib/components/kit/KanjiToken.svelte';
	import { AuthProvider } from '@kavach/ui';
	import { providers } from '$kavach/providers';
	import { onMount } from 'svelte';
	import { signinErrorFrom } from '$lib/auth-landing';

	let selfHost = $state(false);
	let selfHostUrl = $state('dojo.acme.internal');
	let authMsg = $state<string | null>(null);
	let authError = $state(false);

	// kavach's AuthProvider owns the whole sign-in per provider — an OAuth redirect
	// (applying the provider's configured `scopes`, e.g. github → read:org for the
	// F3c org auto-join) or a magic-link email — driven by the `$kavach/providers`
	// list generated from kavach.config. We only surface the outcome message; no
	// per-provider code.
	// A failed sign-in comes back with the reason in the URL (#297): show it. The
	// fragment stays: SvelteKit's replaceState throws until the router has started,
	// which it has not on this first load, and the reason is still true on reload.
	onMount(() => {
		const failed = signinErrorFrom(window.location.hash, window.location.search);
		if (!failed) return;
		authError = true;
		authMsg = failed.message;
	});

	function onAuthError(e: { message?: string } | undefined) {
		authError = true;
		authMsg = e?.message ?? 'Could not sign in.';
	}
	function onAuthSuccess(provider: { mode?: string }) {
		if (provider.mode === 'otp') {
			authError = false;
			authMsg = 'Check your email for the sign-in link.';
		}
	}

	// What sensei is, in three marks (mockup p-signin).
	const FEATURES: [string, string, string][] = [
		['観', 'Watches your sessions', 'locally · on this machine'],
		['己', 'Your rules & guardrails', 'yours to set and edit'],
		['盾', 'Nothing leaves', 'unless you choose to share']
	];
</script>

<div class="bg-paper flex min-h-screen w-full flex-col md:h-screen md:flex-row md:overflow-hidden">
	<!--
		── left · what sensei is ──
		Marketing/insight panel. On phones (<md) it's hidden so the sign-in form is
		primary and never pushed off-screen; on md:+ it takes the fixed 57% split.
		Gradient stays inline (paint/geometry the token system doesn't model — §1.8).
	-->
	<div
		class="border-paper-edge hidden flex-shrink-0 flex-col overflow-auto border-r p-6 md:flex md:w-[57%] md:p-12"
		style="background: linear-gradient(160deg, var(--accent-soft) 0%, var(--paper-soft) 60%)"
	>
		<div class="flex items-center gap-3">
			<KanjiToken char="結" icon={null} size="inherit" toneClass="" class="text-accent text-2xl" />
			<span class="display text-xl" style="letter-spacing: -0.01em">Dōjō</span>
			<span
				class="mono bg-paper border-paper-edge text-ink-mute rounded-full border px-2 py-1 text-xs"
				>dojo.sensei-hq.com</span
			>
		</div>

		<!-- What sensei is (#132, mockup p-signin). It used to show "Acme Corp's"
		     lesson counts and a "+11pp" lift from a fixture — invented numbers in
		     front of every visitor, before sign-in (#241). -->
		<div class="mt-12">
			<div class="text-ink-mute mb-2 text-xs font-semibold uppercase" style="letter-spacing: 0.2em">
				先生 Sensei
			</div>
			<h1 class="display m-0 font-light text-3xl" style="letter-spacing: -0.02em; line-height: 1.08">
				A quiet companion<br />for your<br />AI-assisted work.
			</h1>
			<p class="text-ink-soft mt-4 max-w-[440px] text-sm" style="line-height: 1.6">
				Sensei watches your coding sessions on this machine and surfaces the patterns you're too close to
				see. Yours alone by default — a dōjō is optional, for when you want to share what you learn with a
				team.
			</p>
		</div>

		<div class="mt-8 grid grid-cols-3 gap-3">
			{#each FEATURES as [kanji, title, sub] (kanji)}
				<div class="bg-paper border-paper-edge rounded-lg border p-4">
					<KanjiToken char={kanji} icon={null} size="inherit" toneClass="" class="text-accent text-xl" />
					<div class="text-ink mt-2 text-sm font-semibold" style="line-height: 1.3">{title}</div>
					<div class="text-ink-mute mt-1 text-xs">{sub}</div>
				</div>
			{/each}
		</div>

		<div class="bg-paper border-paper-edge mt-3 flex items-center gap-3 rounded-lg border p-4">
			<KanjiToken char="空" icon={null} size="inherit" toneClass="" class="text-ink-mute text-xl" />
			<div class="min-w-0 flex-1">
				<div class="text-ink text-sm">Sign in and sensei picks up where you left off.</div>
				<div class="text-ink-mute mt-1 text-xs">
					No team, no setup required. <span class="italic">Still listening.</span>
				</div>
			</div>
		</div>

		<div class="flex-1"></div>
		<div class="text-ink-faint mt-6 text-xs" style="line-height: 1.5">
			Local-first · yours by default · join or create a dōjō later only to share with a team.
		</div>
	</div>

	<!-- ── right · sign-in options ── -->
	<div class="flex min-w-0 flex-1 items-center justify-center p-8">
		<div class="w-full max-w-[364px]">
			<h2 class="display m-0 font-normal text-2xl" style="letter-spacing: -0.015em; line-height: 1.1">
				Sign in to continue
			</h2>
			<p class="text-ink-mute mb-6 mt-2 text-sm" style="line-height: 1.55">
				GitHub or Microsoft brings your organizations automatically. Neither? Use a magic link.
			</p>

			<!-- Sign-in providers — kavach's AuthProvider renders each from the config
			     (github and azure OAuth · magic-link email · password); one component,
			     no per-provider code. -->
			<div class="signin-auth flex flex-col gap-3 text-sm">
				{#each providers as provider (provider.name)}
					<AuthProvider
						{...provider}
						onerror={onAuthError}
						onsuccess={() => onAuthSuccess(provider)}
					/>
				{/each}
			</div>
			{#if authMsg}
				<div
					class="mt-3 text-center text-xs {authError ? 'text-danger' : 'text-success'}"
					role="status"
				>
					{authMsg}
				</div>
			{:else}
				<div class="text-ink-faint mt-3 text-center text-xs">
					GitHub or Microsoft brings your orgs; magic link is for everyone else.
				</div>
			{/if}

			<!-- self-hosted -->
			<div class="border-t-paper-edge mt-5 border-t pt-4">
				{#if !selfHost}
					<button
						type="button"
						onclick={() => (selfHost = true)}
						class="text-ink-soft flex w-full cursor-pointer items-center justify-center gap-2 border-none bg-transparent text-sm"
					>
						<KanjiToken char="基" size="inherit" toneClass="" class="text-ink-mute text-sm" />
						Connecting to a self-hosted Dōjō?
						<span class="text-accent">Enter its URL →</span>
					</button>
				{:else}
					<div>
						<label
							for="dojo-selfhost"
							class="text-ink-mute mb-2 block font-semibold uppercase text-xs" style="letter-spacing: 0.1em">Self-hosted Dōjō URL</label
						>
						<div class="flex gap-2">
							<input
								id="dojo-selfhost"
								type="text"
								bind:value={selfHostUrl}
								placeholder="dojo.yourcompany.com"
								class="bg-paper border-paper-edge text-ink box-border flex-1 rounded-lg border px-3 py-3 text-sm"
							/>
							<button
								type="button"
								class="bg-paper border-paper-edge text-ink cursor-pointer whitespace-nowrap rounded-lg border px-4 py-3 text-sm"
								>Connect</button
							>
						</div>
						<div class="text-ink-faint mt-2 text-xs" style="line-height: 1.5">
							Same sign-in — your server authenticates you through GitHub, Microsoft or your email magic
							link on its own domain.
						</div>
					</div>
				{/if}
			</div>

			<div class="text-ink-faint mt-6 text-center text-xs" style="line-height: 1.5">
				One sign-in for the hosted SaaS and any self-hosted Dōjō.
			</div>
			<div class="text-ink-faint mt-3 text-center text-xs" style="line-height: 1.5">
				By signing in you agree to the
				<a href="/terms-of-service" class="text-ink-soft underline">Terms of Service</a>
				and the
				<a href="/privacy-policy" class="text-ink-soft underline">Privacy Policy</a>.
			</div>
		</div>
	</div>
</div>

<style>
	/* Restyle kavach's AuthProvider (rokkit Button/InputField) to the sign-in
	   mockup. Overriding a third-party component's rendered DOM is the one place we
	   reach for token CSS vars instead of uno utilities; scoped by `.signin-auth`,
	   `:global()` reaches the child components. Hooks: `data-auth-mode` (oauth /
	   otp / password), `data-item-icon`, rokkit `data-field-root`. */
	.signin-auth :global([data-auth]) {
		gap: 0.5rem;
	}
	.signin-auth :global([data-auth] button) {
		width: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		border-radius: 0.5rem;
		padding: 0.75rem 1rem;
		font-size: inherit; /* the container's text-sm (#132) */
		font-weight: 500;
		cursor: pointer;
		border: 1px solid var(--paper-edge);
		background: var(--paper);
		color: var(--ink);
	}
	.signin-auth :global([data-auth-mode='oauth'] button) {
		background: var(--primary);
		color: var(--on-primary);
		border-color: transparent;
	}
	.signin-auth :global([data-item-icon]) {
		width: 18px;
		height: 18px;
		flex-shrink: 0;
	}
	.signin-auth :global([data-field-root]) {
		width: 100%;
	}
	.signin-auth :global([data-field] label) {
		display: block;
		font-size: 0.6875rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		color: var(--ink-mute);
		margin-bottom: 0.375rem;
	}
	.signin-auth :global([data-field] input) {
		width: 100%;
		box-sizing: border-box;
		background: var(--paper);
		color: var(--ink);
		border: 1px solid var(--paper-edge);
		border-radius: 0.5rem;
		padding: 0.75rem;
		font-size: inherit; /* the container's text-sm (#132) */
		margin-bottom: 0.5rem;
	}
</style>
