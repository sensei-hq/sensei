<script lang="ts">
	import { getContext } from 'svelte';
	import { metrics } from '$lib/dojo-data';
	import Spark from './Spark.svelte';

	// The browser kavach instance (hydrated in +layout.svelte). Undefined during
	// SSR/prerender, so every use is guarded — the form must render without it.
	const kavach = getContext<Record<string, unknown>>('kavach');

	const m = metrics;

	let email = $state('');
	let selfHost = $state(false);
	let selfHostUrl = $state('dojo.acme.internal');
	let status = $state<'idle' | 'sending' | 'sent' | 'error'>('idle');
	let message = $state('');

	// One generic sign-in for every provider — kavach's whole purpose. It resolves
	// each provider (oauth / magic-link / password) by name from the kavach.config
	// `providers` entry, applying that entry's `scopes` (github requests `read:org`
	// for the F3c org auto-join). No per-provider function: the GitHub button and
	// the magic-link form both call this. OAuth redirects away; OTP stays and
	// confirms "check your email".
	async function signIn(provider: string, opts: Record<string, unknown> = {}) {
		if (status === 'sending') return;
		const fn = kavach?.signIn as
			| ((c: Record<string, unknown>) => Promise<{ error?: { message?: string } }>)
			| undefined;
		if (!fn) return;
		status = 'sending';
		message = '';
		const result = await fn({ provider, ...opts });
		if (result?.error) {
			status = 'error';
			message = result.error.message ?? 'Could not sign in.';
			return;
		}
		if (provider === 'magic') {
			status = 'sent';
			message = 'Check your email for the sign-in link.';
		}
	}

	function submitMagicLink(event: SubmitEvent) {
		event.preventDefault();
		if (!email) return;
		void signIn('magic', { email });
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

			<!-- primary · GitHub OAuth -->
			<button
				type="button"
				onclick={() => signIn('github', { redirectTo: window.location.origin })}
				disabled={status === 'sending'}
				class="bg-primary text-on-primary flex w-full cursor-pointer items-center justify-center gap-3 rounded-lg px-4 py-3 text-sm font-medium disabled:cursor-not-allowed disabled:opacity-55"
			>
				<span class="i-simple-icons:github h-[18px] w-[18px]" aria-hidden="true"></span>
				Continue with GitHub
			</button>
			<div class="text-ink-faint mt-2 text-center text-xs">
				Derives your orgs &amp; roles from GitHub — and matches your repos.
			</div>

			<!-- divider -->
			<div class="my-5 flex items-center gap-3">
				<span class="bg-paper-edge h-px flex-1"></span>
				<span class="mono text-ink-faint text-xs" style="letter-spacing: 0.1em">OR</span>
				<span class="bg-paper-edge h-px flex-1"></span>
			</div>

			<!-- magic link -->
			<form onsubmit={submitMagicLink}>
				<label
					for="dojo-email"
					class="text-ink-mute mb-2 block font-semibold uppercase text-xs" style="letter-spacing: 0.1em">Work email</label
				>
				<input
					id="dojo-email"
					type="email"
					bind:value={email}
					placeholder="you@company.com"
					class="bg-paper border-paper-edge text-ink mb-3 box-border w-full rounded-lg border px-3 py-3 text-sm"
				/>
				<button
					type="submit"
					disabled={status === 'sending'}
					class="bg-paper border-paper-edge text-ink flex w-full cursor-pointer items-center justify-center gap-2 rounded-lg border px-4 py-3 text-sm"
				>
					<span class="kanji text-accent text-sm">鍵</span>
					{status === 'sending' ? 'Sending…' : status === 'sent' ? 'Link sent' : 'Email me a magic link'}
				</button>
			</form>
			{#if message}
				<div
					class="mt-2 text-center text-xs {status === 'error' ? 'text-danger' : 'text-success'}"
					role="status"
				>
					{message}
				</div>
			{:else}
				<div class="text-ink-faint mt-2 text-center text-xs">
					For organizations not on GitHub.
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
