<script lang="ts">
	import { AuthProvider } from '@kavach/ui';
	import { providers } from '$kavach/providers';
	import { metrics } from '$lib/dojo-data';
	import Spark from './Spark.svelte';

	const m = metrics;

	let selfHost = $state(false);
	let selfHostUrl = $state('dojo.acme.internal');
	let authMsg = $state<string | null>(null);
	let authError = $state(false);

	// kavach's AuthProvider owns the whole sign-in per provider — an OAuth redirect
	// (applying the provider's configured `scopes`, e.g. github → read:org for the
	// F3c org auto-join) or a magic-link email — driven by the `$kavach/providers`
	// list generated from kavach.config. We only surface the outcome message; no
	// per-provider code.
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

	const adoptionLift = $derived(Math.round(m.adoptionLift * 100));
</script>

<div class="bg-paper flex min-h-screen w-full flex-col md:h-screen md:flex-row md:overflow-hidden">
	<!--
		── left · welcome back + insight into the Dōjō ──
		Marketing/insight panel. On phones (<md) it's hidden so the sign-in form is
		primary and never pushed off-screen; on md:+ it takes the fixed 57% split.
		Gradient stays inline (paint/geometry the token system doesn't model — §1.8).
	-->
	<div
		class="border-paper-edge hidden flex-shrink-0 flex-col overflow-auto border-r p-6 md:flex md:w-[57%] md:p-12"
		style="background: linear-gradient(160deg, var(--accent-soft) 0%, var(--paper-soft) 60%)"
	>
		<div class="flex items-center gap-3">
			<span class="kanji text-accent text-2xl" style="line-height: 1">結</span>
			<span class="display text-xl" style="letter-spacing: -0.01em">Dōjō</span>
			<span
				class="mono bg-paper border-paper-edge text-ink-mute rounded-full border px-2 py-1 text-xs"
				>dojo.sensei-hq.com</span
			>
		</div>

		<div class="mt-12">
			<div class="text-ink-mute mb-2 text-xs uppercase" style="letter-spacing: 0.2em">
				Welcome back
			</div>
			<h1 class="display m-0 font-light text-3xl" style="letter-spacing: -0.02em; line-height: 1.08">
				Your team kept<br />learning while<br />you were away.
			</h1>
			<p class="text-ink-soft mt-4 max-w-[440px] text-sm" style="line-height: 1.6">
				A snapshot of <b class="font-semibold">Acme Corp's</b> shared mind since your last visit — remembered
				on this device. Sign in to step back in.
			</p>
		</div>

		<div class="mt-8 grid grid-cols-3 gap-3">
			<div class="bg-paper border-paper-edge rounded-xl border p-4">
				<div class="mb-2 flex items-center justify-between">
					<span class="kanji text-accent text-sm">共</span>
					<span class="text-accent"><Spark data={m.contribSpark} /></span>
				</div>
				<div class="display text-ink font-light text-2xl" style="line-height: 1">
					{m.contribWeek}
				</div>
				<div class="text-ink-mute mt-2 text-xs" style="line-height: 1.35">
					lessons shared this week
				</div>
			</div>
			<div class="bg-paper border-paper-edge rounded-xl border p-4">
				<div class="mb-2 flex items-center justify-between">
					<span class="kanji text-accent text-sm">決</span>
				</div>
				<div class="display text-ink font-light text-2xl" style="line-height: 1">
					{m.approvedWeek}
				</div>
				<div class="text-ink-mute mt-2 text-xs" style="line-height: 1.35">
					approved &amp; distributed
				</div>
			</div>
			<div class="bg-paper border-paper-edge rounded-xl border p-4">
				<div class="mb-2 flex items-center justify-between">
					<span class="kanji text-accent text-sm">盾</span>
				</div>
				<div class="display text-ink font-light text-2xl" style="line-height: 1">
					{m.dereferenced}
				</div>
				<div class="text-ink-mute mt-2 text-xs" style="line-height: 1.35">
					anonymized from client work · 0 incidents
				</div>
			</div>
		</div>

		<!-- latest approved teaching -->
		<div class="bg-paper border-paper-edge mt-3 flex items-center gap-4 rounded-xl border p-4">
			<span class="kanji text-accent text-xl">守</span>
			<div class="min-w-0 flex-1">
				<div class="text-ink-faint mb-1 font-semibold uppercase text-xs" style="letter-spacing: 0.12em">
					Just published · Company
				</div>
				<div class="text-ink text-sm">Never log refresh tokens, even at debug level</div>
			</div>
			<div class="text-right">
				<div class="display text-success font-light text-xl">+{adoptionLift}pp</div>
				<div class="text-ink-mute text-xs">first-try resolution</div>
			</div>
		</div>

		<div class="flex-1"></div>
		<div class="text-ink-faint mt-8 text-xs" style="line-height: 1.5">
			Private to your org · governed · anonymized before anything leaves a client engagement.
		</div>
	</div>

	<!-- ── right · sign-in options ── -->
	<div class="flex min-w-0 flex-1 items-center justify-center p-8">
		<div class="w-full max-w-[364px]">
			<h2 class="display m-0 font-normal text-2xl" style="letter-spacing: -0.015em; line-height: 1.1">
				Sign in to continue
			</h2>
			<p class="text-ink-mute mb-6 mt-2 text-sm" style="line-height: 1.55">
				GitHub brings your organizations and roles automatically. No GitHub? Use a magic link.
			</p>

			<!-- Sign-in providers — kavach's AuthProvider renders each from the config
			     (github OAuth with read:org scopes · magic-link email); one component,
			     no per-provider code. -->
			<div class="flex flex-col gap-3">
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
					GitHub brings your orgs &amp; roles; magic link is for orgs not on GitHub.
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
						<span class="kanji text-ink-mute text-sm">基</span>
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
							Same sign-in — your server authenticates you through GitHub (or your email magic link) on
							its own domain.
						</div>
					</div>
				{/if}
			</div>

			<div class="text-ink-faint mt-6 text-center text-xs" style="line-height: 1.5">
				One sign-in for the hosted SaaS and any self-hosted Dōjō.
			</div>
		</div>
	</div>
</div>
