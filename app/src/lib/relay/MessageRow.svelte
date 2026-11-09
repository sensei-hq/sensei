<script lang="ts">
  // One unread crew → person message (Q2 part 2): the sending lane's code,
  // when it was sent, an excerpt, and its task. DecisionRow's visual
  // language, minus Answer — there is no ack tool; the daemon's
  // delivered/read receipts retire the row and the screen re-reads.
  import { since } from './relay-format.js';
  import type { RelayPersonMessage } from '$lib/relay-api.js';

  let {
    message,
    href,
  }: {
    message: RelayPersonMessage;
    /** The task the message is attached to, when it names one. */
    href: string | null;
  } = $props();

  // Needs-you shows the gist; the task view carries the full text.
  const excerpt = $derived(
    message.text.length > 200 ? `${message.text.slice(0, 200)}…` : message.text,
  );
</script>

<article class="rounded-sm border border-paper-edge bg-paper-soft p-3 flex flex-col gap-2" data-person-message>
  <p class="text-sm m-0">
    <span class="font-medium">{message.from_lane_code ?? 'A lane'}</span>
    <span class="text-xs text-ink-faint">· {since(message.sent_at)}</span>
  </p>
  <p class="text-sm text-ink-soft m-0">{excerpt}</p>

  <div class="flex items-center gap-3">
    {#if href}<a class="text-xs underline" href={href}>{message.task_title ?? 'Its task'}</a>{/if}
  </div>
</article>
