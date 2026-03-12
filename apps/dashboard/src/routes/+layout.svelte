<script lang="ts">
	import favicon from '$lib/assets/favicon.svg';

	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<nav>
  <a href="/">Dashboard</a>
  <a href="/stats">Stats</a>
  <a href="/repos">Repos</a>
  <a href="/benchmarks">Benchmarks</a>
  <a href="/libraries">Libraries</a>
  <a href="/references">References</a>
</nav>

{@render children()}
