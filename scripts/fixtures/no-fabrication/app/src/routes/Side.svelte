<script lang="ts">
	// Fixture: T4 hard-coded live indicators and T1 inside a component script.
	import { api } from '$lib/api';
	const data = api.health().catch(() => null); // expect: T1
</script>

<footer>daemon · running</footer> <!-- expect: T4 -->
<span class="pill">Connected</span> <!-- expect: T4 -->
<!-- <span>Running</span> a comment, not markup -->
<span>{status.label}</span>
