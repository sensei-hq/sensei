<script lang="ts">
  // A report waiting for a reviewer (queue cutover, phase 4): the task, who
  // wrote it, which turn, and how long it has floated. Any lane but the
  // author claims it next; the person only watches it here.
  import type { RelayFloatingReview } from '$lib/relay-api.js';
  import { since } from './relay-format.js';

  let { review, href }: { review: RelayFloatingReview; href: string } = $props();

  const author = $derived(
    review.author_lane_title
      ? `${review.author_lane_code} · ${review.author_lane_title}`
      : review.author_lane_code,
  );
</script>

<div
  class="border-t border-paper-edge py-2 flex flex-wrap items-center gap-2 text-sm"
  data-floating-review
  data-task-id={review.task_id}
>
  <a class="hover:underline" data-review-title {href}>{review.title}</a>
  <span class="text-xs text-ink-mute" data-author>{author}</span>
  <span class="text-xs text-ink-faint" data-turn>{review.phase} · r{review.round}</span>
  <span class="ml-auto text-xs text-ink-faint" data-since>{since(review.reported_at)}</span>
</div>
