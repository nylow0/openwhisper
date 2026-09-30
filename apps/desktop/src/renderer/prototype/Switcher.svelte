<script lang="ts">
  // PROTOTYPE: floating variant switcher. ←/→ cycle variants (ignored while typing).
  import { createEventDispatcher, onMount } from 'svelte';

  export let variants: Array<{ key: string; name: string }>;
  export let current: string;
  export let empty: boolean;
  export let lightBackdrop: boolean;

  const dispatch = createEventDispatcher<{
    change: string;
    toggleEmpty: void;
    toggleBackdrop: void;
    error: void;
  }>();

  $: index = Math.max(0, variants.findIndex((variant) => variant.key === current));
  $: label = variants[index];

  function step(delta: number) {
    dispatch('change', variants[(index + delta + variants.length) % variants.length].key);
  }

  onMount(() => {
    function onKey(event: KeyboardEvent) {
      const target = event.target instanceof Element ? event.target : null;
      if (target?.closest('input, textarea, [contenteditable]')) return;
      if (event.key === 'ArrowLeft') step(-1);
      if (event.key === 'ArrowRight') step(1);
    }
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });
</script>

<div
  class="fixed bottom-5 left-1/2 z-[100] flex -translate-x-1/2 items-center gap-1 rounded-full bg-white p-1 font-sans text-[12px] font-semibold text-black shadow-[0_8px_40px_rgba(255,255,255,0.15)]"
>
  <button class="rounded-full px-3 py-1.5 hover:bg-black/10" on:click={() => step(-1)} aria-label="Previous variant">←</button>
  <span class="min-w-[150px] text-center">{label.key} · {label.name}</span>
  <button class="rounded-full px-3 py-1.5 hover:bg-black/10" on:click={() => step(1)} aria-label="Next variant">→</button>
  <span class="mx-1 h-4 w-px bg-black/20"></span>
  <button class="rounded-full px-3 py-1.5 hover:bg-black/10" on:click={() => dispatch('toggleEmpty')}>
    {empty ? 'Sample data' : 'Empty state'}
  </button>
  <button class="rounded-full px-3 py-1.5 hover:bg-black/10" on:click={() => dispatch('error')}>Error toast</button>
  <button class="rounded-full px-3 py-1.5 hover:bg-black/10" on:click={() => dispatch('toggleBackdrop')}>
    Overlay on {lightBackdrop ? 'dark' : 'light'} app
  </button>
</div>
