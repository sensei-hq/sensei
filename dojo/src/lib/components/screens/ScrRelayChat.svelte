<script lang="ts">
	import { SectionHead, ChatThread, EmptyState, Icon } from '$lib/components/kit';
	import type { KitChatTurn, KitMe } from '$lib/components/kit/types';

	// Relay · chat (mockup ScrRelayChat) — the sensei-speaks-rarely thread plus a
	// reply input. Presentational: the page supplies the thread + viewer. `onSend`
	// fires with a non-empty trimmed reply; the input clears on send. sensei stays
	// quiet — the thread carries the mentor voice, the viewer replies.
	//
	// When there is no active run/thread (the honest, common case with no relay
	// data), the screen degrades to the shared EmptyState — no fabricated project
	// header + empty thread. The head + thread + reply input only render when a real
	// run is in flight (a non-empty thread, with a project to name it).
	let {
		thread = [],
		me,
		project,
		session = 's-2891',
		onSend
	}: {
		thread?: KitChatTurn[];
		me?: KitMe;
		project?: string;
		session?: string;
		onSend?: (text: string) => void;
	} = $props();

	let reply = $state('');

	const hasThread = $derived(thread.length > 0);

	function send() {
		const text = reply.trim();
		if (!text) return;
		onSend?.(text);
		reply = '';
	}
</script>

<div class="flex flex-col p-8 gap-6">
	{#if hasThread}
		<SectionHead eyebrow={'Relay · chat · ' + session} title={project ?? 'active session'} />

		<div class="bg-paper-soft border-paper-edge rounded-lg border p-6">
			<ChatThread {thread} {me} />
		</div>

		<form
			class="bg-paper-soft border-paper-edge flex items-center gap-2 rounded-lg border py-1 pr-1 pl-3"
			onsubmit={(e) => {
				e.preventDefault();
				send();
			}}
		>
			<Icon name="chat-round-line" size={16} toneClass="text-ink-mute" />
			<input
				bind:value={reply}
				type="text"
				placeholder="reply to sensei…"
				class="text-ink placeholder:text-ink-faint h-9 flex-1 bg-transparent text-sm outline-none"
			/>
			<button
				type="submit"
				aria-label="Send reply"
				class="text-on-primary bg-accent flex h-9 w-9 flex-shrink-0 cursor-pointer items-center justify-center rounded"
			>
				<Icon name="arrow-right" size={16} toneClass="text-on-primary" />
			</button>
		</form>
	{:else}
		<EmptyState kanji="話" title="No active session.">
			When a run is in flight you can steer it here — sensei surfaces its reasoning and you reply. It
			is quiet until then.
		</EmptyState>
	{/if}
</div>
