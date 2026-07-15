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

	async function sendMagicLink(event: SubmitEvent) {
		event.preventDefault();
		if (!email || status === 'sending') return;
		const signIn = kavach?.signIn as
			| ((c: { provider: string; email: string }) => Promise<{ error?: { message?: string } }>)
			| undefined;
		if (!signIn) return;
		status = 'sending';
		message = '';
		const result = await signIn({ provider: 'magic', email });
		if (result?.error) {
			status = 'error';
			message = result.error.message ?? 'Could not send the magic link.';
		} else {
			status = 'sent';
			message = 'Check your email for the sign-in link.';
		}
	}

	const adoptionLift = $derived(Math.round(m.adoptionLift * 100));
</script>

<div class="bg-paper flex h-screen w-full overflow-hidden">
	<!-- ── left · welcome back + insight into the Dōjō ── -->
	<div
		class="border-paper-edge flex flex-shrink-0 flex-col overflow-auto border-r"
		style="width: 57%; padding: 44px 52px; background: linear-gradient(160deg, var(--accent-soft) 0%, var(--paper-soft) 60%)"
	>
		<div class="flex items-center gap-3">
			<span class="kanji text-accent" style="font-size: 26px; line-height: 1">結</span>
			<span class="display text-xl" style="letter-spacing: -0.01em">Dōjō</span>
			<span
				class="mono bg-paper border-paper-edge text-ink-mute rounded-full border text-xs"
				style="padding: 3px 10px">dojo.sensei-hq.com</span
			>
		</div>

		<div style="margin-top: 48px">
			<div class="text-ink-mute text-xs uppercase" style="letter-spacing: 0.2em; margin-bottom: 10px">
				Welcome back
			</div>
			<h1 class="display font-light" style="font-size: 42px; letter-spacing: -0.02em; margin: 0; line-height: 1.08">
				Your team kept<br />learning while<br />you were away.
			</h1>
			<p class="text-ink-soft text-sm" style="line-height: 1.6; margin: 16px 0 0; max-width: 440px">
				A snapshot of <b class="font-semibold">Acme Corp's</b> shared mind since your last visit — remembered
				on this device. Sign in to step back in.
			</p>
		</div>

		<div class="grid" style="grid-template-columns: 1fr 1fr 1fr; gap: 12px; margin-top: 34px">
			<div class="bg-paper border-paper-edge rounded-xl border" style="padding: 15px 16px">
				<div class="flex items-center justify-between" style="margin-bottom: 9px">
					<span class="kanji text-accent text-sm">共</span>
					<span class="text-accent"><Spark data={m.contribSpark} /></span>
				</div>
				<div class="display text-ink font-light" style="font-size: 30px; line-height: 1">
					{m.contribWeek}
				</div>
				<div class="text-ink-mute text-xs" style="margin-top: 7px; line-height: 1.35">
					lessons shared this week
				</div>
			</div>
			<div class="bg-paper border-paper-edge rounded-xl border" style="padding: 15px 16px">
				<div class="flex items-center justify-between" style="margin-bottom: 9px">
					<span class="kanji text-accent text-sm">決</span>
				</div>
				<div class="display text-ink font-light" style="font-size: 30px; line-height: 1">
					{m.approvedWeek}
				</div>
				<div class="text-ink-mute text-xs" style="margin-top: 7px; line-height: 1.35">
					approved &amp; distributed
				</div>
			</div>
			<div class="bg-paper border-paper-edge rounded-xl border" style="padding: 15px 16px">
				<div class="flex items-center justify-between" style="margin-bottom: 9px">
					<span class="kanji text-accent text-sm">盾</span>
				</div>
				<div class="display text-ink font-light" style="font-size: 30px; line-height: 1">
					{m.dereferenced}
				</div>
				<div class="text-ink-mute text-xs" style="margin-top: 7px; line-height: 1.35">
					anonymized from client work · 0 incidents
				</div>
			</div>
		</div>

		<!-- latest approved teaching -->
		<div
			class="bg-paper border-paper-edge flex items-center gap-4 rounded-xl border"
			style="margin-top: 14px; padding: 15px 17px"
		>
			<span class="kanji text-accent" style="font-size: 22px">守</span>
			<div class="flex-1" style="min-width: 0">
				<div
					class="text-ink-faint font-semibold uppercase"
					style="font-size: 10.5px; letter-spacing: 0.12em; margin-bottom: 3px"
				>
					Just published · Company
				</div>
				<div class="text-ink text-sm">Never log refresh tokens, even at debug level</div>
			</div>
			<div class="text-right">
				<div class="display text-success font-light" style="font-size: 22px">+{adoptionLift}pp</div>
				<div class="text-ink-mute" style="font-size: 10px">first-try resolution</div>
			</div>
		</div>

		<div class="flex-1"></div>
		<div class="text-ink-faint text-xs" style="margin-top: 28px; line-height: 1.5">
			Private to your org · governed · anonymized before anything leaves a client engagement.
		</div>
	</div>

	<!-- ── right · sign-in options ── -->
	<div class="flex flex-1 items-center justify-center" style="min-width: 0; padding: 40px">
		<div style="width: 364px; max-width: 100%">
			<h2 class="display font-normal" style="font-size: 26px; letter-spacing: -0.015em; margin: 0; line-height: 1.1">
				Sign in to continue
			</h2>
			<p class="text-ink-mute text-sm" style="line-height: 1.55; margin: 8px 0 26px">
				GitHub brings your organizations and roles automatically. No GitHub? Use a magic link.
			</p>

			<!-- primary · GitHub (OAuth wiring deferred to a later chunk) -->
			<button
				type="button"
				disabled
				class="bg-primary text-on-primary flex w-full items-center justify-center gap-3 rounded-lg text-sm font-medium"
				style="padding: 13px 18px; cursor: not-allowed; opacity: 0.55"
			>
				<span class="i-auth-github" aria-hidden="true" style="width: 18px; height: 18px"></span>
				Continue with GitHub
			</button>
			<div class="text-ink-faint text-center text-xs" style="margin-top: 7px">
				Derives your orgs &amp; roles from GitHub — and matches your repos.
			</div>

			<!-- divider -->
			<div class="flex items-center gap-3" style="margin: 20px 0">
				<span class="bg-paper-edge flex-1" style="height: 1px"></span>
				<span class="mono text-ink-faint" style="font-size: 10.5px; letter-spacing: 0.1em">OR</span>
				<span class="bg-paper-edge flex-1" style="height: 1px"></span>
			</div>

			<!-- magic link -->
			<form onsubmit={sendMagicLink}>
				<label
					for="dojo-email"
					class="text-ink-mute block font-semibold uppercase"
					style="font-size: 11px; letter-spacing: 0.1em; margin-bottom: 7px">Work email</label
				>
				<input
					id="dojo-email"
					type="email"
					bind:value={email}
					placeholder="you@company.com"
					class="bg-paper border-paper-edge text-ink w-full rounded-lg border text-sm"
					style="box-sizing: border-box; padding: 11px 13px; margin-bottom: 10px"
				/>
				<button
					type="submit"
					disabled={status === 'sending'}
					class="bg-paper border-paper-edge text-ink flex w-full items-center justify-center gap-2 rounded-lg border text-sm"
					style="padding: 12px 18px; cursor: pointer"
				>
					<span class="kanji text-accent text-sm">鍵</span>
					{status === 'sending' ? 'Sending…' : status === 'sent' ? 'Link sent' : 'Email me a magic link'}
				</button>
			</form>
			{#if message}
				<div
					class="text-center text-xs {status === 'error' ? 'text-danger' : 'text-success'}"
					style="margin-top: 7px"
					role="status"
				>
					{message}
				</div>
			{:else}
				<div class="text-ink-faint text-center text-xs" style="margin-top: 7px">
					For organizations not on GitHub.
				</div>
			{/if}

			<!-- self-hosted -->
			<div style="margin-top: 22px; padding-top: 18px; border-top: 1px solid var(--paper-edge)">
				{#if !selfHost}
					<button
						type="button"
						onclick={() => (selfHost = true)}
						class="text-ink-soft flex w-full items-center justify-center gap-2"
						style="background: none; border: none; cursor: pointer; font-size: 12.5px"
					>
						<span class="kanji text-ink-mute" style="font-size: 13px">基</span>
						Connecting to a self-hosted Dōjō?
						<span class="text-accent">Enter its URL →</span>
					</button>
				{:else}
					<div>
						<label
							for="dojo-selfhost"
							class="text-ink-mute block font-semibold uppercase"
							style="font-size: 11px; letter-spacing: 0.1em; margin-bottom: 7px">Self-hosted Dōjō URL</label
						>
						<div class="flex gap-2">
							<input
								id="dojo-selfhost"
								type="text"
								bind:value={selfHostUrl}
								placeholder="dojo.yourcompany.com"
								class="bg-paper border-paper-edge text-ink flex-1 rounded-lg border text-sm"
								style="box-sizing: border-box; padding: 11px 13px"
							/>
							<button
								type="button"
								class="bg-paper border-paper-edge text-ink rounded-lg border text-sm"
								style="padding: 12px 16px; white-space: nowrap; cursor: pointer">Connect</button
							>
						</div>
						<div class="text-ink-faint text-xs" style="margin-top: 8px; line-height: 1.5">
							Same sign-in — your server authenticates you through GitHub (or your email magic link) on
							its own domain.
						</div>
					</div>
				{/if}
			</div>

			<div class="text-ink-faint text-center text-xs" style="margin-top: 24px; line-height: 1.5">
				One sign-in for the hosted SaaS and any self-hosted Dōjō.
			</div>
		</div>
	</div>
</div>
