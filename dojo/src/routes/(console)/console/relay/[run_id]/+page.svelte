<script lang="ts">
	import { invalidateAll } from '$app/navigation';
	import ConsoleHead from '$lib/components/ConsoleHead.svelte';
	import DojoChip from '$lib/components/DojoChip.svelte';
	import RelayStatusBadge from '$lib/components/RelayStatusBadge.svelte';
	import {
		submitReview,
		sendNudge,
		DojoApiError,
		type RelaySegment,
		type SegmentReview
	} from '$lib/relay-data';
	import { segmentStateBadge } from '$lib/relay-view';

	// Relay run detail (mockup relay-planner.jsx RelayPlan + dojo-relay.jsx WatchPhases):
	// a run's segment outline as a Phase → Step tree, with a PR-review-style verdict
	// draft per segment that is held locally and flushed in one "Send review". The load
	// (+page.ts → getSegments + listRuns) does the fetching; this component is
	// presentational plus the single submitReview mutation. tenantKey/accessToken come
	// from the (console) layout server load, merged onto `data`. Mirrors triage's
	// on-demand-action pattern (busy flag, DojoApiError-aware catch, invalidateAll).
	let { data } = $props();

	type Verdict = SegmentReview['verdict'];
	interface Draft {
		verdict?: Verdict;
		note?: string;
	}

	// Local review draft, keyed by segment seq. Nothing is sent until Send review.
	let drafts = $state<Record<number, Draft>>({});

	let sending = $state(false);
	let sendError = $state<string | null>(null);

	// "Nudge the run" composer — an unsolicited steer sent TO the held run
	// (sendNudge → a new relay_inbox row, distinct from the PR-review batch). Same
	// mutation shape as submitReview: busy flag, DojoApiError-aware catch → inline
	// non-fatal error, plus a transient success line that clears on the next send.
	let nudgeText = $state('');
	let nudging = $state(false);
	let nudgeError = $state<string | null>(null);
	let nudgeSent = $state(false);
	const canNudge = $derived(nudgeText.trim().length > 0);

	async function nudge() {
		if (!canNudge || nudging) return;
		nudging = true;
		nudgeError = null;
		nudgeSent = false;
		try {
			await sendNudge(data.tenantKey, data.runId, nudgeText.trim(), {
				fetch,
				accessToken: data.accessToken
			});
			nudgeText = '';
			nudgeSent = true;
		} catch (e) {
			nudgeError = e instanceof DojoApiError ? e.message : 'could not send the nudge';
		} finally {
			nudging = false;
		}
	}

	// Phase → Step tree from the flat, seq-ordered segment list. Top-level segments
	// (parent_id === null) are phases; the rest hang under their parent, keeping the
	// load's seq order within each group. Orphans (a parent that isn't a top-level
	// phase) are surfaced at the top level so nothing is silently dropped.
	interface PhaseNode {
		phase: RelaySegment;
		steps: RelaySegment[];
	}
	const outline = $derived.by<PhaseNode[]>(() => {
		const phases: PhaseNode[] = [];
		const byId: Record<string, PhaseNode> = {};
		for (const seg of data.segments) {
			if (seg.parent_id === null) {
				const node: PhaseNode = { phase: seg, steps: [] };
				phases.push(node);
				byId[seg.id] = node;
			}
		}
		for (const seg of data.segments) {
			if (seg.parent_id === null) continue;
			const parent = byId[seg.parent_id];
			if (parent) parent.steps.push(seg);
			else phases.push({ phase: seg, steps: [] }); // orphan — show standalone
		}
		return phases;
	});

	// The batch to send: one SegmentReview per draft that has a chosen verdict. The
	// note is trimmed and dropped when empty so an accidental blank note isn't sent.
	const pending = $derived.by<SegmentReview[]>(() => {
		const out: SegmentReview[] = [];
		for (const [seq, draft] of Object.entries(drafts)) {
			if (!draft.verdict) continue;
			const note = draft.note?.trim();
			out.push({ seq: Number(seq), verdict: draft.verdict, ...(note ? { note } : {}) });
		}
		return out;
	});

	// Set (or clear, when re-clicking the same one) the drafted verdict for a segment.
	function setVerdict(seq: number, verdict: Verdict) {
		const current = drafts[seq] ?? {};
		drafts[seq] = { ...current, verdict: current.verdict === verdict ? undefined : verdict };
	}

	function setNote(seq: number, note: string) {
		drafts[seq] = { ...(drafts[seq] ?? {}), note };
	}

	const done = $derived(data.segments.filter((s) => s.state === 'done').length);
	const total = $derived(data.segments.length);

	async function send() {
		if (pending.length === 0) return;
		sending = true;
		sendError = null;
		try {
			await submitReview(data.tenantKey, data.runId, pending, {
				fetch,
				accessToken: data.accessToken
			});
			drafts = {};
			await invalidateAll();
		} catch (e) {
			sendError = e instanceof DojoApiError ? e.message : 'could not send the review';
		} finally {
			sending = false;
		}
	}

	// The three verdict affordances, in the order the mockup reviews read.
	const VERDICTS: { verdict: Verdict; kanji: string; label: string }[] = [
		{ verdict: 'approve', kanji: '許', label: 'Approve' },
		{ verdict: 'request_changes', kanji: '直', label: 'Request changes' },
		{ verdict: 'comment', kanji: '言', label: 'Comment' }
	];

	function verdictLabel(v: string): string {
		return VERDICTS.find((x) => x.verdict === v)?.label ?? v;
	}
</script>

<svelte:head>
	<title>{data.run?.title ?? 'Run'} · Relay · Dōjō console</title>
</svelte:head>

<div class="bg-paper flex h-full w-full flex-col overflow-hidden">
	<ConsoleHead
		kanji="継"
		eyebrow="Relay · run"
		title={data.run?.title ?? 'Run'}
		sub={data.run?.goal ?? 'A supervised run — review its plan segment by segment, then send in one pass.'}
	>
		{#snippet right()}
			<div class="flex items-center gap-3">
				{#if data.run}
					<RelayStatusBadge status={data.run.status} />
					<span class="mono text-ink-mute text-xs">{done}/{total}</span>
				{/if}
				<button
					type="button"
					onclick={send}
					disabled={sending || pending.length === 0}
					class="bg-ink text-on-primary inline-flex items-center gap-2 rounded-lg text-xs font-medium"
					style="padding: 8px 13px; border: none"
					style:opacity={sending || pending.length === 0 ? 0.5 : 1}
					style:cursor={sending || pending.length === 0 ? 'not-allowed' : 'pointer'}
				>
					<span class="kanji" style="font-size: 12px">送</span>
					{#if sending}
						Sending review…
					{:else}
						Send review{#if pending.length > 0}
							<span class="mono" style="opacity: 0.7">· {pending.length} to send</span>
						{/if}
					{/if}
				</button>
			</div>
		{/snippet}
	</ConsoleHead>

	<div class="flex-1 overflow-auto" style="padding: 8px 28px 28px">
		{#if data.error}
			<div
				class="bg-warning-soft border-warning-edge text-ink-soft flex items-center gap-2 rounded-xl border text-sm"
				style="padding: 12px 16px; margin-top: 16px"
			>
				<span class="kanji text-warning">検</span>
				<span
					>Live relay is unavailable. <span class="mono text-ink-mute text-xs">{data.error}</span></span
				>
			</div>
		{/if}

		{#if sendError}
			<div
				class="bg-warning-soft border-warning-edge text-ink-soft flex items-center gap-2 rounded-xl border text-sm"
				style="padding: 12px 16px; margin-top: 16px"
			>
				<span class="kanji text-warning">検</span>
				<span>Review not sent. <span class="mono text-ink-mute text-xs">{sendError}</span></span>
			</div>
		{/if}

		{#if data.segments.length === 0 && !data.error}
			<div
				class="border-ink-faint text-ink-mute flex flex-col items-center gap-2 rounded-xl border border-dashed text-center"
				style="padding: 48px 28px; margin-top: 24px"
			>
				<span class="kanji text-ink-faint" style="font-size: 30px">継</span>
				<div class="text-ink-soft text-sm">No outline yet</div>
				<div class="text-ink-mute text-xs">The run hasn't published its plan.</div>
			</div>
		{/if}

		{#if outline.length > 0}
			<div class="flex flex-col gap-3" style="margin-top: 18px">
				{#each outline as node (node.phase.id)}
					{@const phaseBadge = segmentStateBadge(node.phase.state)}
					<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-xl border">
						<!-- Phase header -->
						<div
							class="flex items-start gap-3 {node.steps.length > 0 ? 'border-paper-edge border-b' : ''}"
							style="padding: 14px 16px"
						>
							<span class="mono text-ink-faint flex-shrink-0 text-xs" style="width: 22px; padding-top: 2px"
								>{String(node.phase.seq).padStart(2, '0')}</span
							>
							<div class="flex-1" style="min-width: 0">
								<div class="flex items-center gap-2">
									<div class="text-ink text-sm font-semibold" style="flex: 1; min-width: 0">
										{node.phase.title}
									</div>
									{#if node.phase.is_gate}
										{@const blocking = node.phase.gate_severity === 'blocking'}
										<DojoChip toneClass={blocking ? 'text-accent' : 'text-ink-mute'}>
											{blocking ? 'Gate · needs you' : 'Gate · heads-up'}
										</DojoChip>
									{/if}
									<DojoChip toneClass={phaseBadge.toneClass}>{phaseBadge.label}</DojoChip>
								</div>
								{#if node.phase.summary}
									<div class="text-ink-mute text-xs" style="margin-top: 4px; line-height: 1.5">
										{node.phase.summary}
									</div>
								{/if}
							</div>
						</div>

						<!-- Steps -->
						{#each node.steps as step, i (step.id)}
							{@const stepBadge = segmentStateBadge(step.state)}
							{@const draft = drafts[step.seq]}
							{@const blocking = step.gate_severity === 'blocking'}
							<div
								class="{i < node.steps.length - 1 ? 'border-paper-edge border-b' : ''}"
								style="padding: 13px 16px 13px 40px"
							>
								<div class="flex items-start gap-3">
									<div class="flex-1" style="min-width: 0">
										<div class="flex flex-wrap items-center gap-2">
											<span class="text-ink text-sm" style="flex: 1; min-width: 0">{step.title}</span>
											{#if step.is_gate}
												<DojoChip toneClass={blocking ? 'text-accent' : 'text-ink-mute'}>
													{blocking ? 'Gate · needs you' : 'Gate · heads-up'}
												</DojoChip>
											{/if}
											<DojoChip toneClass={stepBadge.toneClass}>{stepBadge.label}</DojoChip>
										</div>
										{#if step.summary}
											<div class="text-ink-mute text-xs" style="margin-top: 4px; line-height: 1.5">
												{step.summary}
											</div>
										{/if}
										{#if step.detail}
											<div class="text-ink-faint text-xs" style="margin-top: 4px; line-height: 1.5">
												{step.detail}
											</div>
										{/if}

										{#if step.response_verdict && step.submitted_at}
											<!-- Already reviewed — prior verdict shown; re-review still allowed. -->
											<div
												class="bg-success-soft border-success-edge text-ink-soft inline-flex items-center gap-2 rounded-lg border text-xs"
												style="padding: 5px 10px; margin-top: 8px"
											>
												<span class="kanji text-success" style="font-size: 12px">済</span>
												<span>Reviewed · <b class="font-semibold">{verdictLabel(step.response_verdict)}</b></span>
												{#if step.response_note}
													<span class="text-ink-mute">— {step.response_note}</span>
												{/if}
											</div>
										{/if}

										<!-- PR-review affordances: Approve / Request changes / Comment + note. -->
										<div class="flex flex-wrap items-center gap-2" style="margin-top: 9px">
											{#each VERDICTS as v (v.verdict)}
												{@const active = draft?.verdict === v.verdict}
												<button
													type="button"
													onclick={() => setVerdict(step.seq, v.verdict)}
													aria-pressed={active}
													class="inline-flex items-center gap-1 rounded-lg text-xs font-medium {active
														? 'bg-ink text-on-primary'
														: 'bg-paper border-paper-edge text-ink-soft border'}"
													style="padding: 6px 11px; cursor: pointer"
													style:border={active ? 'none' : undefined}
												>
													<span class="kanji" style="font-size: 11px">{v.kanji}</span>
													{v.label}
												</button>
											{/each}
										</div>

										{#if draft?.verdict === 'request_changes' || draft?.verdict === 'comment'}
											<textarea
												value={draft?.note ?? ''}
												oninput={(e) => setNote(step.seq, e.currentTarget.value)}
												placeholder={draft.verdict === 'request_changes'
													? 'What needs to change?'
													: 'Add a comment (optional)…'}
												rows="2"
												class="bg-paper border-paper-edge text-ink w-full rounded-lg border text-sm"
												style="padding: 8px 11px; margin-top: 8px; resize: vertical; font-family: inherit; line-height: 1.5"
											></textarea>
										{/if}
									</div>
								</div>
							</div>
						{/each}
					</div>
				{/each}
			</div>
		{/if}

		{#if data.run && !data.error}
			<!-- Nudge the run — an unsolicited steer sent to the held run. -->
			<div
				class="bg-paper-soft border-paper-edge rounded-xl border"
				style="padding: 15px 18px; margin-top: 18px"
			>
				<div class="flex items-center gap-2">
					<span class="kanji text-accent" style="font-size: 13px">促</span>
					<span
						class="text-ink-mute text-xs font-semibold"
						style="letter-spacing: 0.14em; text-transform: uppercase">Nudge the run</span
					>
				</div>
				<div class="text-ink-mute text-xs" style="margin-top: 4px; line-height: 1.5">
					Steer it mid-flight — sensei picks the note up on its next check.
				</div>

				<textarea
					aria-label="Nudge the run"
					bind:value={nudgeText}
					placeholder="Steer the run — e.g. 'focus on the API first'"
					rows="2"
					disabled={nudging}
					class="bg-paper border-paper-edge text-ink w-full rounded-lg border text-sm"
					style="padding: 8px 11px; margin-top: 10px; resize: vertical; font-family: inherit; line-height: 1.5"
				></textarea>

				{#if nudgeError}
					<div class="text-danger text-xs" style="margin-top: 8px">
						Nudge not sent. <span class="mono text-ink-mute">{nudgeError}</span>
					</div>
				{/if}

				<div class="flex items-center gap-3" style="margin-top: 10px">
					<button
						type="button"
						onclick={nudge}
						disabled={nudging || !canNudge}
						class="bg-ink text-on-primary inline-flex items-center gap-2 rounded-lg text-xs font-medium"
						style="padding: 8px 13px; border: none"
						style:opacity={nudging || !canNudge ? 0.5 : 1}
						style:cursor={nudging || !canNudge ? 'not-allowed' : 'pointer'}
					>
						<span class="kanji" style="font-size: 12px">送</span>
						{nudging ? 'Sending…' : 'Send'}
					</button>
					{#if nudgeSent}
						<span class="text-success inline-flex items-center gap-1 text-xs">
							<span class="kanji text-success" style="font-size: 11px">済</span>
							Nudge sent
						</span>
					{/if}
				</div>
			</div>
		{/if}
	</div>
</div>
