<script lang="ts">
  // A form decision's yes/no (#389): rokkit's InputCheckbox, drawn native. Its default
  // custom look hides the input and puts a focusable icon under aria-hidden, which axe
  // refuses (aria-hidden-focus, serious); the native box is labelled and keyboard-reachable.
  import { InputCheckbox } from '@rokkit/forms';

  let { value = $bindable(), ...rest }: { value?: boolean | null } & Record<string, unknown> = $props();
</script>

<InputCheckbox bind:value {...rest} variant="default" />
