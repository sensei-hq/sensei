<script lang="ts">
	// A read that failed, said as such (#335): announced to assistive tech, and
	// shown in place of the screen's empty state — a failed read is never an
	// empty list. `what` names what could not be loaded ("the triage queue").
	let { what, error }: { what: string; error: string } = $props();
</script>

<p role="alert" class="text-sm text-danger m-0">Could not load {what} — {error}</p>
