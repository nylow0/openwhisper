<script lang="ts">
  // On/off switch. The track fills with the blue→cyan accent when on.
  import { createEventDispatcher } from 'svelte';

  export let checked: boolean;
  export let label: string;
  export let disabled = false;

  const dispatch = createEventDispatcher<{ change: boolean }>();
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  class="relative h-5 w-9 shrink-0 rounded-full transition-colors disabled:opacity-50 {checked ? 'bg-accent' : 'bg-ink-700'}"
  on:click={() => dispatch('change', !checked)}
>
  <span
    class="absolute top-[3px] h-[14px] w-[14px] rounded-full bg-white shadow transition-all {checked
      ? 'left-[19px]'
      : 'left-[3px] opacity-60'}"
  ></span>
</button>
