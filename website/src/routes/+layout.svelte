<script lang="ts">
  import 'uno.css';
  import '../app.css';
  import { vibe } from '@rokkit/states';
  import { themable } from '@rokkit/actions';

  let { children } = $props();
</script>

<svelte:head>
  <title>Sensei — A quiet companion for AI-assisted work</title>
  <meta name="description" content="Sensei watches your sessions with AI assistants, then surfaces the patterns you're too close to see. Local-first. Free during preview." />
</svelte:head>
<svelte:body use:themable={{ theme: vibe, storageKey: 'sensei-site-theme' }} />

{@render children()}
