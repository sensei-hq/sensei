<script lang="ts">
	import Icon from './Icon.svelte';
	import Chip from './Chip.svelte';
	import Btn from './Btn.svelte';
	import type { KitGate } from './types';

	// A gate card (kit K2GateCard) — a command awaiting approve / deny. Header:
	// project · risk chip (danger tint when high) · age. Body: the command in a
	// mono well, the why + session line, and the action row (approve once · always
	// allow · deny). `onApprove` / `onDeny` fire the caller's handlers.
	let {
		gate,
		onApprove,
		onDeny
	}: { gate: KitGate; onApprove?: () => void; onDeny?: () => void } = $props();

	const high = $derived(gate.risk === 'high');
</script>

<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-lg border">
	<div class="border-paper-edge flex items-center gap-2 border-b" style="padding: 12px 16px">
		<Icon name="command" size={18} toneClass="text-accent" />
		<span class="text-ink flex-1 text-sm font-medium">{gate.project}</span>
		<Chip
			mono
			toneClass={high ? 'text-danger' : 'text-warning'}
			softClass={high ? 'bg-danger-soft' : 'bg-warning-soft'}
			edgeClass={high ? 'border-danger-edge' : 'border-warning-soft'}>{gate.risk}</Chip
		>
		<span class="mono text-ink-faint text-xs">{gate.age}</span>
	</div>
	<div style="padding: 16px">
		<div
			class="mono text-ink bg-paper-mute border-paper-edge rounded border text-sm"
			style="padding: 12px; overflow-x: auto"
		>
			$ {gate.cmd}
		</div>
		<div class="text-ink-soft text-sm" style="line-height: 1.55; margin-top: 8px">
			{gate.why} · session {gate.session}
		</div>
		<div class="flex flex-wrap items-center gap-2" style="margin-top: 12px">
			<Btn size="sm" icon="check-circle" onclick={onApprove}>Approve once</Btn>
			<Btn size="sm" variant="ghost" onclick={onApprove}>Always allow</Btn>
			<span class="flex-1"></span>
			<Btn size="sm" variant="ghost" icon="close-circle" onclick={onDeny}>Deny</Btn>
		</div>
	</div>
</div>
