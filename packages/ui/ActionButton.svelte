<script lang="ts">
  // One next step, as a rokkit Button (a link when it has `href`). Shared by
  // Notice, EmptyState and Banner so a step looks the same wherever it is offered.
  import { Button } from '@rokkit/ui';
  import type { NoticeAction } from './types.js';

  let {
    action,
    look = 'solid',
    name = 'primary',
  }: {
    action: NoticeAction;
    look?: 'solid' | 'outline' | 'link';
    /** The `data-action` hook when the action names none. */
    name?: string;
  } = $props();
</script>

<Button
  label={action.label}
  size="sm"
  variant={look === 'solid' ? 'primary' : 'default'}
  style={look === 'solid' ? 'default' : look}
  href={action.href}
  target={action.href && action.external ? '_blank' : undefined}
  {...(action.href && action.external ? { rel: 'noopener noreferrer' } : {})}
  disabled={action.busy}
  onclick={action.onclick}
  data-action={action.name ?? name}
/>
