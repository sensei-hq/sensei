<script lang="ts">
	import { relativeAge } from '$lib/triage-view';
	import { replyToGate, DojoApiError, type RelayGate } from '$lib/relay-data';

	// Relay "needs you" gate card (mockup relay.jsx RelayApprove/RelayRespond +
	// dojo-relay.jsx RelayApproveBody/RelayDecisionBody): one pending gate the agent
	// is waiting on the human for, in plain language with approve/decline (or a
	// decision's options, or a free-text reply) plus an optional note. Never renders
	// code or diffs — the payload carries a stripped prompt by design. Presentational
	// plus the single replyToGate mutation; mirrors the run-detail Send pattern (busy
	// $state, DojoApiError-aware catch → inline error line, onReplied() on success).
	let {
		gate,
		tenantKey,
		accessToken,
		onReplied
	}: {
		gate: RelayGate;
		tenantKey: string;
		accessToken: string | null;
		onReplied?: () => void;
	} = $props();

	const ZERO_UUID = '00000000-0000-0000-0000-000000000000';

	// The stripped prompt, or a friendly fallback when the payload carries none.
	const prompt = $derived(
		typeof gate.payload.prompt === 'string' && gate.payload.prompt.trim()
			? gate.payload.prompt
			: 'The run needs a decision'
	);

	// A decision gate's options, when present and non-empty (rokkit form schema).
	const options = $derived.by<string[]>(() => {
		const raw = gate.payload.options;
		if (!Array.isArray(raw)) return [];
		return raw.filter((o): o is string => typeof o === 'string');
	});

	// Deep-link to the run only when it's a real run id (not the all-zeros uuid).
	const runHref = $derived(
		gate.run_id && gate.run_id !== ZERO_UUID ? `/console/relay/${gate.run_id}` : null
	);

	let note = $state('');
	let freeText = $state('');
	let busy = $state(false);
	let replyError = $state<string | null>(null);

	// Fold an optional trimmed note onto whatever reply body the affordance built.
	function withNote(reply: Record<string, unknown>): Record<string, unknown> {
		const trimmed = note.trim();
		return trimmed ? { ...reply, note: trimmed } : reply;
	}

	async function send(reply: Record<string, unknown>) {
		if (busy) return;
		busy = true;
		replyError = null;
		try {
			await replyToGate(tenantKey, gate.id, withNote(reply), { fetch, accessToken });
			onReplied?.();
		} catch (e) {
			replyError = e instanceof DojoApiError ? e.message : 'could not send your reply';
		} finally {
			busy = false;
		}
	}
</script>

<div
	class="bg-accent-soft border-accent-edge rounded-xl border"
	style="padding: 15px 18px"
>
	<div class="flex items-start gap-3">
		<span
			class="kanji text-accent flex-shrink-0 text-center"
			style="font-size: 20px; width: 24px; line-height: 1.1">要</span
		>
		<div class="flex-1" style="min-width: 0">
			<div class="text-ink text-sm font-medium" style="line-height: 1.5">{prompt}</div>
			<div class="flex items-center gap-2" style="margin-top: 4px">
				{#if runHref}
					<a href={runHref} class="text-ink-mute truncate text-xs no-underline hover:underline">
						{gate.run_title ?? 'View run'}
					</a>
				{:else if gate.run_title}
					<span class="text-ink-mute truncate text-xs">{gate.run_title}</span>
				{/if}
				<span class="text-ink-faint text-xs">·</span>
				<span class="text-ink-faint text-xs">{relativeAge(gate.created_at)}</span>
			</div>
		</div>
	</div>

	{#if replyError}
		<div class="text-danger text-xs" style="margin-top: 10px">
			Reply not sent. <span class="mono text-ink-mute">{replyError}</span>
		</div>
	{/if}

	<!-- Optional note carried along with any reply. -->
	<textarea
		bind:value={note}
		placeholder="Add a note (optional)…"
		rows="2"
		disabled={busy}
		class="bg-paper border-paper-edge text-ink w-full rounded-lg border text-sm"
		style="padding: 8px 11px; margin-top: 12px; resize: vertical; font-family: inherit; line-height: 1.5"
	></textarea>

	{#if options.length > 0}
		<!-- Decision: one button per option → { choice: <option> }. -->
		<div class="flex flex-wrap items-center gap-2" style="margin-top: 10px">
			{#each options as option (option)}
				<button
					type="button"
					onclick={() => send({ choice: option })}
					disabled={busy}
					class="bg-ink text-on-primary inline-flex items-center gap-1 rounded-lg text-xs font-medium"
					style="padding: 8px 13px; border: none"
					style:opacity={busy ? 0.5 : 1}
					style:cursor={busy ? 'not-allowed' : 'pointer'}
				>
					{option}
				</button>
			{/each}
		</div>
	{:else if gate.kind === 'approval'}
		<!-- Approval: Approve / Decline → { verdict: 'approve' | 'deny' }. -->
		<div class="flex flex-wrap items-center gap-2" style="margin-top: 10px">
			<button
				type="button"
				onclick={() => send({ verdict: 'approve' })}
				disabled={busy}
				class="bg-ink text-on-primary inline-flex items-center gap-1 rounded-lg text-xs font-medium"
				style="padding: 8px 13px; border: none"
				style:opacity={busy ? 0.5 : 1}
				style:cursor={busy ? 'not-allowed' : 'pointer'}
			>
				<span class="kanji text-accent" style="font-size: 11px">許</span>
				{busy ? 'Sending…' : 'Approve'}
			</button>
			<button
				type="button"
				onclick={() => send({ verdict: 'deny' })}
				disabled={busy}
				class="bg-paper border-paper-edge text-ink-soft inline-flex items-center gap-1 rounded-lg border text-xs font-medium"
				style="padding: 8px 13px"
				style:opacity={busy ? 0.5 : 1}
				style:cursor={busy ? 'not-allowed' : 'pointer'}
			>
				{busy ? 'Sending…' : 'Decline'}
			</button>
		</div>
	{:else}
		<!-- Free-text: a small input → { text: <value> } with Send. -->
		<div class="flex items-center gap-2" style="margin-top: 10px">
			<input
				type="text"
				bind:value={freeText}
				placeholder="Type your answer…"
				disabled={busy}
				class="bg-paper border-paper-edge text-ink flex-1 rounded-lg border text-sm"
				style="padding: 8px 11px; min-width: 0; font-family: inherit"
			/>
			<button
				type="button"
				onclick={() => send({ text: freeText })}
				disabled={busy || freeText.trim() === ''}
				class="bg-ink text-on-primary inline-flex items-center gap-1 rounded-lg text-xs font-medium"
				style="padding: 8px 13px; border: none"
				style:opacity={busy || freeText.trim() === '' ? 0.5 : 1}
				style:cursor={busy || freeText.trim() === '' ? 'not-allowed' : 'pointer'}
			>
				<span class="kanji" style="font-size: 11px">送</span>
				{busy ? 'Sending…' : 'Send'}
			</button>
		</div>
	{/if}
</div>
