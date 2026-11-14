<script lang="ts">
  // The bootstrap's "Run this in your terminal" card — the shared Notice's panel,
  // in the action tone (it is fine once the reader runs it). Every other screen's
  // errors now look like this one (docs/architecture/notice.md).
  import type { Remedy } from '$lib/health-types.js';
  import { Notice } from '@sensei/ui';

  interface Props {
    remedy: Remedy;
    /** Optional override for clipboard write. Tests inject a mock; production
     *  defaults to `navigator.clipboard.writeText`. */
    writeText?: (text: string) => Promise<void>;
    onVerify?: () => void;
  }
  let { remedy, writeText, onVerify }: Props = $props();
</script>

<div data-component="remedy" class="mt-4">
  <Notice
    size="panel"
    tone="action"
    title="Run this in your terminal"
    script={remedy.script}
    {writeText}
    primary={{ label: "I've run it · re-check", onclick: onVerify, name: 'verify' }}
    secondary={remedy.url ? { label: 'Learn more', href: remedy.url, external: true, name: 'learn-more' } : undefined}
  />
</div>
