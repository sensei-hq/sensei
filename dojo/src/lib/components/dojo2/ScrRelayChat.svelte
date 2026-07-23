<script lang="ts">
	import { SectionHead, ChatThread, Icon } from '$lib/components/kit';
	import type { KitChatTurn, KitMe } from '$lib/components/kit/types';

	// Relay · chat (mockup ScrRelayChat) — the sensei-speaks-rarely thread plus a
	// reply input. Presentational: the page supplies the thread + viewer (kit
	// fixtures this chunk). `onSend` fires with a non-empty trimmed reply; the
	// input clears on send. sensei stays quiet — the thread carries the mentor
	// voice, the viewer replies.
	let {
		thread = [],
		me,
		project = 'lumen-auth',
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

	function send() {
		const text = reply.trim();
		if (!text) return;
		onSend?.(text);
		reply = '';
	}
</script>

<div class="flex flex-col p-8 gap-6">
	<SectionHead eyebrow={'Relay · chat · ' + session} title={project} />

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
			class="text-ink placeholder:text-ink-faint flex-1 bg-transparent text-sm"
			style="border: none; outline: none; height: 34px"
		/>
		<button
			type="submit"
			aria-label="Send reply"
			class="text-on-primary bg-accent flex flex-shrink-0 cursor-pointer items-center justify-center rounded"
			style="width: 34px; height: 34px"
		>
			<Icon name="arrow-right" size={16} toneClass="text-on-primary" />
		</button>
	</form>
</div>
