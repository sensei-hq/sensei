<script lang="ts">
  // Mounted by ModalShell.spec.svelte.ts only. `ctl` is a reactive object so a
  // test can open and close the shell after mount.
  import ModalShell from './ModalShell.svelte';

  let { ctl, onClose }: { ctl: { open: boolean }; onClose: () => void } = $props();
</script>

<button type="button" data-testid="opener">open</button>
<ModalShell open={ctl.open} component="harness-modal" labelledBy="harness-title" {onClose}>
  <h2 id="harness-title">Title</h2>
  <button type="button" disabled data-testid="disabled">disabled</button>
  <button type="button" data-testid="first">first</button>
  <a href="#x" data-testid="link">link</a>
  <button type="button" data-testid="last">last</button>
</ModalShell>
