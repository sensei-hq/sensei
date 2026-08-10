<script lang="ts">
  import 'uno.css';
  import '../app.css';
  import { vibe } from '@rokkit/states';
  import { themable } from '@rokkit/actions';

  // zen-sumi is our brand style, but the vibe store only allows rokkit/minimal/
  // material by default — register it, then make it the site default. themable's
  // load() still honours a returning visitor's stored choice.
  vibe.allowedStyles = ['zen-sumi', ...vibe.allowedStyles];
  vibe.style = 'zen-sumi';

  let { children } = $props();
</script>

<svelte:body use:themable={{ theme: vibe, storageKey: 'sensei-site-theme' }} />

{@render children()}
