<script lang="ts">
  import 'uno.css';
  import '../app.css';
  import { vibe } from '@rokkit/states';
  import { themable } from '@rokkit/actions';

  let { children } = $props();
</script>

<svelte:head>
  <title>Sensei</title>
  <meta name="description" content="AI Development Intelligence — capture how your team uses AI and feed that intelligence back into your workflow." />
</svelte:head>
<svelte:body use:themable={{ theme: vibe, storageKey: 'sensei-theme' }} />

{@render children()}
