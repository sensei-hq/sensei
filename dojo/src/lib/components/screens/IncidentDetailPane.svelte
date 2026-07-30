<script lang="ts">
	import { Chip, Icon, Btn } from '$lib/components/kit';
	import { severityTone } from '$lib/incidents-view';
	import type { KitIncidentDetail } from '$lib/components/kit/types';

	// The incident "Open" detail pane (GET …/incidents/{id}) — beyond the list row:
	// the resolved owner, SLA, resolution note, and the linked near-leak artifact.
	// Net-new (no mockup); a design-system card matching the console panels.
	// Presentational — the page fetches the detail and passes it (null = hidden).
	let { detail, onClose }: { detail: KitIncidentDetail | null; onClose?: () => void } = $props();
</script>

{#if detail}
	{@const sv = severityTone(detail.severity)}
	<div class="bg-paper-soft border-paper-edge rounded-lg border" style="padding: 16px">
		<div class="flex items-center gap-3" style="margin-bottom: 12px">
			<div class="text-ink-mute flex-1 text-xs font-semibold uppercase" style="letter-spacing: 0.18em">
				Incident detail
			</div>
			{#if onClose}
				<Btn size="sm" variant="ghost" icon="close-circle" onclick={onClose}>Close</Btn>
			{/if}
		</div>

		<div class="text-ink text-sm font-medium">{detail.title}</div>
		<div class="flex flex-wrap items-center gap-2" style="margin-top: 6px">
			<Chip toneClass={sv.text} softClass={sv.soft} edgeClass={sv.edge}>{detail.severity}</Chip>
			<span class="text-ink-faint mono text-xs">
				{detail.client} · {detail.state} · opened {detail.opened}
			</span>
		</div>

		<div class="flex flex-col gap-2" style="margin-top: 12px">
			<div class="text-ink-soft text-sm">Owner · {detail.owner}</div>
			{#if detail.sla}
				<div class="text-ink-soft text-sm">SLA due · {detail.sla}</div>
			{/if}
			{#if detail.resolution}
				<div class="text-ink-soft flex items-center gap-2 text-sm">
					<Icon name="check-circle" size={15} toneClass="text-success" />{detail.resolution}
				</div>
			{/if}
			{#if detail.artifact}
				<div class="text-ink-soft flex items-center gap-2 text-sm">
					<Icon name="document" size={15} toneClass="text-ink-mute" />
					{detail.artifact.title} · {detail.artifact.kind} · {detail.artifact.status}
				</div>
			{/if}
		</div>
	</div>
{/if}
