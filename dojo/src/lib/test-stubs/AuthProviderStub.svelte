<script lang="ts">
	// Minimal stand-in for @kavach/ui's AuthProvider under vitest (the real one
	// pulls @rokkit/ui + @rokkit/forms + the kavach context). Renders the provider
	// label so DojoSignIn render specs can assert the buttons appear.
	let { name, label }: { name: string; label?: string } = $props();
</script>

<button type="button" data-auth-provider={name}>{label}</button>
