<script lang="ts">
	// A read that failed, said as such (#335): announced to assistive tech, and
	// shown in place of the screen's empty state — a failed read is never an
	// empty list. `what` names what could not be loaded ("the triage queue").
	// Rendered by the shared Notice (docs/architecture/notice.md), so it reads the
	// same as a failure anywhere in sensei or the dōjō.
	import { Notice } from '@sensei/ui';

	let { what, error, onretry }: { what: string; error: string | null | undefined; onretry?: () => void } = $props();
</script>

<Notice
	size="panel"
	tone="fault"
	title="Couldn't load {what}"
	detail={error || 'The dōjō gave no answer.'}
	primary={onretry ? { label: 'Retry', onclick: onretry, name: 'retry' } : undefined}
	data-load-error
/>
