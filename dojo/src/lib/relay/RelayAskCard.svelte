<script lang="ts">
	import { Icon } from '$lib/components/kit';
	import { Button } from '@rokkit/ui';
	import { relativeAge } from '$lib/triage/view';
	import type { RelayAsk, AskAction } from './types';
	import * as m from '$lib/paraglide/messages';

	// One ask (mockup AskCard) — the run holds here until you answer. Per spec Q3 the
	// kind glyph is a Solar icon (not kanji) + a verb label; per Q4 the options are a
	// selectable rokkit-schema-style list (click to pick) with one optional freeform
	// line, not "type 1–N". Answered → a verdict echo card. Pure over props; the reply
	// routes out through onanswer (state.answerAsk now, POST /relay/reply later).
	let {
		ask,
		verdict,
		onanswer
	}: {
		ask: RelayAsk;
		verdict?: string;
		onanswer?: (askId: string, verdict: string) => void;
	} = $props();

	// action → Solar icon + verb label (status ≠ action: a stalled run takes "resume").
	const KIND: Record<AskAction, { icon: string; label: string }> = {
		approve: { icon: 'check-circle', label: m.ask_approve() },
		choose: { icon: 'checklist-minimalistic', label: m.ask_choose() },
		resume: { icon: 'restart', label: m.ask_resume() },
		chat: { icon: 'chat-round-line', label: m.ask_chat() }
	};
	const kind = $derived(KIND[ask.action]);
	const holds = $derived(ask.taskTitle ?? ask.segmentId ?? null);

	let picked = $state<number | null>(null);
	let freeform = $state('');
	const ready = $derived(picked !== null || freeform.trim().length > 0);

	function pick(i: number) {
		picked = picked === i ? null : i;
		if (picked !== null) freeform = '';
	}
	function onFreeform(e: Event) {
		freeform = (e.currentTarget as HTMLInputElement).value;
		if (freeform.trim()) picked = null;
	}
	function send() {
		if (!ready) return;
		onanswer?.(ask.id, picked !== null ? ask.options[picked] : freeform.trim());
	}
</script>

{#if verdict}
	<!-- Answered — the verdict echo (mockup 了 card, Solar check instead of kanji). -->
	<div class="bg-paper-soft border-paper-edge flex items-start gap-3 rounded-lg border p-4">
		<Icon name="check-circle" size={20} toneClass="text-success" />
		<div class="min-w-0 flex-1">
			<div class="text-ink-mute text-sm">{ask.prompt}</div>
			<div class="text-ink text-sm" style="margin-top: 2px">{m.ask_answered()} · {verdict}</div>
		</div>
	</div>
{:else}
	<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-lg border">
		<!-- Header band — kind icon · verb label · blocking · age · question · context · holds. -->
		<div class="bg-accent-soft border-paper-edge flex items-start gap-3 border-b p-4">
			<Icon name={kind.icon} size={20} toneClass="text-accent" />
			<div class="min-w-0 flex-1">
				<div class="flex flex-wrap items-center gap-2">
					<span class="text-accent text-xs font-medium uppercase" style="letter-spacing: 0.18em">{kind.label}</span>
					{#if ask.blocking}
						<span class="mono border-accent-soft text-accent rounded-full border text-xs" style="padding: 0 7px; line-height: 16px">{m.ask_blocking()}</span>
					{/if}
					<span class="flex-1"></span>
					<span class="mono text-ink-faint text-xs">{relativeAge(ask.createdAt)}</span>
				</div>
				<div class="text-ink text-sm font-medium" style="margin-top: 3px; line-height: 1.4">{ask.prompt}</div>
				{#if ask.context}
					<div class="text-ink-soft text-sm" style="margin-top: 2px; line-height: 1.5">{ask.context}</div>
				{/if}
				{#if holds}
					<div class="mono text-ink-mute text-xs" style="margin-top: 4px">{m.ask_holds({ task: holds })}</div>
				{/if}
			</div>
		</div>
		<!-- Answer — selectable options + one freeform line + send. -->
		<div class="flex flex-col gap-2 px-4 pb-4 pt-3">
			{#if ask.options.length}
				<div class="flex flex-col" style="gap: 2px">
					{#each ask.options as opt, i (opt)}
						{@const on = picked === i}
						<button
							type="button"
							onclick={() => pick(i)}
							aria-pressed={on}
							class="flex items-center gap-3 rounded px-3 py-2 text-left {on
								? 'bg-paper-mute border-accent'
								: 'border-transparent'} border"
						>
							<span class="mono shrink-0 text-xs {on ? 'text-accent' : 'text-ink-faint'}">{i + 1}</span>
							<span class="text-sm {on ? 'text-ink' : 'text-ink-soft'}">{opt}</span>
						</button>
					{/each}
				</div>
			{/if}
			<div class="flex items-center gap-2">
				<div class="border-paper-edge flex flex-1 items-center gap-2 rounded-lg border bg-paper px-3" style="height: 38px; min-width: 0">
					<span class="mono text-ink-faint shrink-0 text-xs">›</span>
					<input
						value={freeform}
						oninput={onFreeform}
						onkeydown={(e) => e.key === 'Enter' && send()}
						placeholder={m.ask_placeholder()}
						class="text-ink text-sm"
						style="flex: 1; min-width: 0; background: transparent; border: none; outline: none"
					/>
				</div>
				<Button
					variant="primary"
					size="sm"
					icon="check-circle"
					disabled={!ready}
					onclick={send}
					class="shrink-0"
				>
					{m.ask_send()}
				</Button>
			</div>
			<span class="mono text-ink-faint text-xs">{m.ask_hold_note()}</span>
		</div>
	</div>
{/if}
