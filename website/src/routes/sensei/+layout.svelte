<script lang="ts">
  let { children } = $props();
</script>

<svelte:head>
  <title>Sensei — A quiet companion for AI-assisted work</title>
  <meta name="description" content="Sensei watches your sessions with AI assistants, then surfaces the patterns you're too close to see. Local-first. Free during preview." />
</svelte:head>

{@render children()}
