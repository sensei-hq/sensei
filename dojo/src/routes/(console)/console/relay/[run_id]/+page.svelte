<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '@rokkit/ui';
	import { invalidateAll } from '$app/navigation';
	import ConsoleHead from '$lib/components/ConsoleHead.svelte';
	import DojoChip from '$lib/components/DojoChip.svelte';
	import RelayStatusBadge from '$lib/components/RelayStatusBadge.svelte';
	import RelayOfflineBanner from '$lib/components/RelayOfflineBanner.svelte';
	import RelayGateCard from '$lib/components/RelayGateCard.svelte';
	import {
		submitReview,
		sendNudge,
		replyToGate,
		DojoApiError,
		type RelaySegment,
		type SegmentReview,
		type RelayGate
	} from '$lib/relay-data';
	import {
		memoryStore,
		loadDrafts,
		saveDraft,
		clearDrafts,
		enqueue,
		peekAll,
		size,
		flushQueue,
		type KeyValueStore,
		type QueuedAction,
		type ActionSender
	} from '$lib/relay-offline';
	import { isOnline, watchConnectivity } from '$lib/relay-connectivity';
	import { segmentStateBadge } from '$lib/relay-view';
	import { env } from '$env/dynamic/public';
	import { subscribeRelay } from '$lib/relay-realtime';

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
	// A calm, non-fatal confirmation shown when a send/nudge is HELD locally (offline
	// or a mid-send network drop) rather than delivered — cleared on the next real send.
	let sendQueuedNote = $state<string | null>(null);

	// ── Offline / reconnect / draft-queue (P4.5) ─────────────────────────────────
	// Connectivity + the local draft/queue store. `store` is built client-side only
	// (never at SSR/module eval — localStorage doesn't exist on the server); it
	// wraps window.localStorage when present, else an in-memory fallback so callers
	// never null-check. `online` keeps its $state default true so SSR renders online;
	// onMount seeds it from isOnline() and watchConnectivity keeps it live.
	let store: KeyValueStore | undefined = $state();
	let online = $state(true);
	let queuedCount = $state(0);
	let flushing = $state(false);
	let flushNote = $state<string | null>(null);

	/** Re-read the queued count for THIS run (call after any enqueue / flush). */
	function refreshQueued() {
		if (store) queuedCount = size(store, data.runId);
	}

	// "Nudge the run" composer — an unsolicited steer sent TO the held run
	// (sendNudge → a new relay_inbox row, distinct from the PR-review batch). Same
	// mutation shape as submitReview: busy flag, DojoApiError-aware catch → inline
	// non-fatal error, plus a transient success line that clears on the next send.
	let nudgeText = $state('');
	let nudging = $state(false);
	let nudgeError = $state<string | null>(null);
	let nudgeSent = $state(false);
	const canNudge = $derived(nudgeText.trim().length > 0);

	// The composer is progressive: collapsed to a compact trigger so the run
	// detail isn't anchored by an empty box. Expands on demand and collapses
	// again once a nudge lands.
	let showNudge = $state(false);

	// Queue the current nudge locally (offline, or a mid-send network drop). Shared
	// by the offline short-circuit and the network-failure catch so both behave the
	// same: hold it, confirm calmly, collapse the composer, refresh the count.
	function queueNudge() {
		if (!store) return;
		enqueue(store, { kind: 'nudge', runId: data.runId, payload: { text: nudgeText.trim() } });
		nudgeText = '';
		showNudge = false;
		sendQueuedNote = "nudge queued — it'll send when you reconnect";
		refreshQueued();
	}

	async function nudge() {
		if (!canNudge || nudging) return;
		nudging = true;
		nudgeError = null;
		nudgeSent = false;
		// Offline: hold the nudge locally rather than fail — it flushes on reconnect.
		if (!online) {
			queueNudge();
			nudging = false;
			return;
		}
		try {
			await sendNudge(data.tenantKey, data.runId, nudgeText.trim(), {
				fetch,
				accessToken: data.accessToken
			});
			nudgeText = '';
			nudgeSent = true;
			showNudge = false;
			sendQueuedNote = null;
		} catch (e) {
			// A DojoApiError is a real API rejection — surface it inline. Anything else
			// is a network failure (offline mid-send) — hold the nudge and flush later.
			if (e instanceof DojoApiError) {
				nudgeError = e.message;
			} else {
				queueNudge();
			}
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

	// This run's pending "needs you" gates (dojo.relay_inbox rows) — the live-answer
	// affordance. Filtered to this run in the load; each is answered via RelayGateCard's
	// replyToGate mutation, and onReplied → invalidateAll refreshes the list.
	// Offline follow-up: RelayGateCard calls replyToGate directly, so an offline reply
	// surfaces the card's inline error rather than queueing — wiring gate-reply into the
	// P4.5 offline queue (its `reply` dispatch already exists) is a tracked follow-up.
	const gates = $derived<RelayGate[]>(data.gates);

	// Queue the current review batch locally (offline, or a mid-send network drop).
	// Drafts are intentionally NOT cleared here — they stay until the entry actually
	// flushes, so a re-open still shows the pending review.
	function queueReview() {
		if (!store) return;
		enqueue(store, { kind: 'review', runId: data.runId, payload: { reviews: pending } });
		sendQueuedNote = "review queued — it'll send when you reconnect";
		refreshQueued();
	}

	async function send() {
		if (pending.length === 0) return;
		sending = true;
		sendError = null;
		// Offline: hold the batch locally rather than fail — it flushes on reconnect.
		if (!online) {
			queueReview();
			sending = false;
			return;
		}
		try {
			await submitReview(data.tenantKey, data.runId, pending, {
				fetch,
				accessToken: data.accessToken
			});
			drafts = {};
			if (store) clearDrafts(store, data.runId);
			sendQueuedNote = null;
			await invalidateAll();
		} catch (e) {
			// A DojoApiError is a real API rejection — surface it inline. Anything else
			// is a network failure (offline mid-send) — hold the batch and flush later.
			if (e instanceof DojoApiError) {
				sendError = e.message;
			} else {
				queueReview();
			}
		} finally {
			sending = false;
		}
	}

	// Flush every queued mutation for this run through the real relay-data senders.
	// Runs on reconnect (via watchConnectivity) and from the banner's "Send queued".
	// Each sender is wrapped so a DojoApiError OR a network error becomes a kept
	// {ok:false} (flushQueue leaves failures queued for a later retry); a success is
	// {ok:true} and its entry is removed durably. Partial-failure safe — never throws.
	async function flush() {
		if (!store || !online || flushing) return;
		flushing = true;
		const sender: ActionSender = async (entry: QueuedAction) => {
			try {
				if (entry.kind === 'review') {
					await submitReview(data.tenantKey, entry.runId, entry.payload.reviews, {
						fetch,
						accessToken: data.accessToken
					});
				} else if (entry.kind === 'nudge') {
					await sendNudge(data.tenantKey, entry.runId, entry.payload.text, {
						fetch,
						accessToken: data.accessToken,
						kind: entry.payload.kind
					});
				} else {
					await replyToGate(data.tenantKey, entry.payload.inboxId, entry.payload.reply, {
						fetch,
						accessToken: data.accessToken
					});
				}
				return { ok: true } as const;
			} catch (e) {
				const error = e instanceof Error ? e.message : 'send failed';
				return { ok: false, error } as const;
			}
		};
		try {
			const { sent, failed } = await flushQueue(store, peekAll(store, data.runId), sender);
			refreshQueued();
			if (failed.length === 0 && sent.length > 0) {
				drafts = {};
				clearDrafts(store, data.runId);
				flushNote = null;
				sendQueuedNote = null;
				await invalidateAll();
			} else if (failed.length > 0) {
				flushNote = "some changes couldn't send yet — still queued";
			}
		} finally {
			flushing = false;
		}
	}

	onMount(() => {
		// Build the storage adapter CLIENT-SIDE only: window.localStorage when it
		// exists (private mode / disabled storage → fall back to in-memory so the page
		// never throws). Never touched at module/SSR eval time.
		let ls: KeyValueStore | undefined;
		try {
			if (typeof window !== 'undefined' && window.localStorage) ls = window.localStorage;
		} catch {
			ls = undefined; // storage blocked (private mode) — fall through to memory.
		}
		store = ls ?? memoryStore();

		// Rehydrate any held drafts BEFORE anything can edit — seed the review drafts
		// and the nudge composer from the last-persisted bundle. Doing this first
		// means the persist effect (below) can't clobber a fresh rehydrate.
		const held = loadDrafts(store, data.runId);
		if (Object.keys(held.segments).length > 0) drafts = { ...held.segments };
		if (held.nudge) nudgeText = held.nudge;
		refreshQueued();

		// Offline note: with no live connection the page still shows the last-loaded
		// outline/inbox that SvelteKit's load data retained — the durable server rows
		// (dojo.relay_segments / relay_inbox) remain the source of truth. A deeper
		// offline mirror (IndexedDB snapshot of the outline) is a deferred follow-up,
		// out of scope for P4.5 v1; here we only buffer drafts + the outbound queue.

		// Seed live connectivity, then keep it in sync. A reconnect edge flushes.
		online = isOnline();
		const connectivityTeardown = watchConnectivity((isOn, reconnected) => {
			online = isOn;
			if (reconnected) flush();
		});

		// P4.2 realtime swap: subscribe to Supabase Realtime on the user's relay rows
		// so this run's outline/inbox refresh LIVE (a new gate/segment/progress update
		// arrives without a user action). subscribeRelay authorizes as the user
		// (data.accessToken → session JWT) so RLS scopes it to their own runs; onChange
		// debounces an invalidateAll. It also serves as a reconnect-refresh signal — a
		// redundant invalidateAll alongside P4.5's online/offline flush is harmless. A
		// no-op under SSR / when unauthenticated. Scoped to this run's topic so its
		// channel is distinct per run.
		const realtimeTeardown = subscribeRelay({
			url: env.PUBLIC_SUPABASE_URL,
			anonKey: env.PUBLIC_SUPABASE_ANON_KEY,
			accessToken: data.accessToken,
			topic: `relay:run:${data.runId}`,
			onChange: () => invalidateAll()
		});

		// Tear down BOTH on unmount — no leaked listeners or channels on navigate-away.
		return () => {
			connectivityTeardown();
			realtimeTeardown();
		};
	});

	// Persist drafts as the user edits, debounced ~300ms. Reads `drafts`/`nudgeText`
	// (its dependencies) and writes the whole per-run bundle. Guarded so it no-ops
	// until the store is ready; the timeout is cleared on re-run/unmount via teardown.
	$effect(() => {
		if (!store) return;
		const s = store;
		const snapshot = { segments: { ...drafts }, nudge: nudgeText.trim() ? nudgeText : undefined };
		const timer = setTimeout(() => saveDraft(s, data.runId, snapshot), 300);
		return () => clearTimeout(timer);
	});

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
				<Button
					variant="primary"
					size="sm"
					onclick={send}
					disabled={sending || pending.length === 0}
				>
					<span class="kanji" style="font-size: 12px">送</span>
					{#if sending}
						Sending review…
					{:else}
						Send review{#if pending.length > 0}
							<span class="mono" style="opacity: 0.7">· {pending.length} to send</span>
						{/if}
					{/if}
				</Button>
			</div>
		{/snippet}
	</ConsoleHead>

	<div class="flex-1 overflow-auto" style="padding: 8px 28px 28px">
		<RelayOfflineBanner {online} queued={queuedCount} {flushing} onSend={flush} note={flushNote} />

		{#if gates.length > 0}
			<!-- Needs you — this run's pending gate(s), answerable in place. Sits at the
				 top of the run body so the away-from-keyboard loop doesn't dead-end: a
				 gate surfaced in the blocked-home can be replied to right here. -->
			<div
				class="bg-paper-soft border-paper-edge rounded-xl border"
				style="padding: 15px 18px; margin-top: 16px"
			>
				<div class="flex items-center gap-2">
					<span class="kanji text-accent" style="font-size: 13px">要</span>
					<span
						class="text-ink-mute text-xs font-semibold"
						style="letter-spacing: 0.14em; text-transform: uppercase">Needs you</span
					>
				</div>
				<div class="flex flex-col gap-3" style="margin-top: 12px">
					{#each gates as g (g.id)}
						<RelayGateCard
							gate={g}
							tenantKey={data.tenantKey}
							accessToken={data.accessToken}
							onReplied={() => invalidateAll()}
						/>
					{/each}
				</div>
			</div>
		{/if}

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

		{#if sendQueuedNote}
			<!-- Held-locally confirmation — a calm success tone, not an error. -->
			<div
				class="bg-success-soft border-success-edge text-ink-soft flex items-center gap-2 rounded-xl border text-sm"
				style="padding: 12px 16px; margin-top: 16px"
			>
				<span class="kanji text-success" style="font-size: 12px">済</span>
				<span>{sendQueuedNote}</span>
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

										<!-- PR-review affordances: Approve / Request changes / Comment + note.
											 A segmented toggle — the drafted verdict reads as a filled primary. -->
										<div class="flex flex-wrap items-center gap-2" style="margin-top: 9px">
											{#each VERDICTS as v (v.verdict)}
												{@const active = draft?.verdict === v.verdict}
												<Button
													variant={active ? 'primary' : 'default'}
													style={active ? 'default' : 'outline'}
													size="sm"
													onclick={() => setVerdict(step.seq, v.verdict)}
													aria-pressed={active}
												>
													<span class="kanji" style="font-size: 11px">{v.kanji}</span>
													{v.label}
												</Button>
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
			<!-- Nudge the run — an unsolicited steer sent to the held run. Collapsed
				 to a compact trigger by default; the composer expands on demand. -->
			<div
				class="bg-paper-soft border-paper-edge rounded-xl border"
				style="padding: 15px 18px; margin-top: 18px"
			>
				{#if !showNudge}
					<!-- Collapsed: a calm trigger + any lingering "sent" confirmation. -->
					<div class="flex items-center gap-3">
						<Button style="ghost" size="sm" onclick={() => (showNudge = true)}>
							<span class="kanji text-accent" style="font-size: 13px">促</span>
							Nudge the run
						</Button>
						{#if nudgeSent}
							<span class="text-success inline-flex items-center gap-1 text-xs">
								<span class="kanji text-success" style="font-size: 11px">済</span>
								Nudge sent
							</span>
						{/if}
					</div>
				{:else}
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
						<Button variant="primary" size="sm" onclick={nudge} disabled={nudging || !canNudge}>
							<span class="kanji" style="font-size: 12px">送</span>
							{nudging ? 'Sending…' : 'Send'}
						</Button>
						<Button style="link" size="sm" onclick={() => (showNudge = false)} disabled={nudging}>
							Cancel
						</Button>
					</div>
				{/if}
			</div>
		{/if}
	</div>
</div>
