<script lang="ts">
  // PROTOTYPE: stand-in for Electron's titleBarOverlay caption buttons (minimize / maximize / close).
  export let color = '#d4d4d8';
  export let height = 44;
</script>

<div class="flex h-full shrink-0" style="color: {color}; height: {height}px;">
  {#each ['min', 'max', 'close'] as button}
    <div
      class="flex w-[46px] items-center justify-center opacity-80 {button === 'close'
        ? 'hover:bg-[#c42b1c] hover:text-white hover:opacity-100'
        : 'hover:bg-white/10'}"
    >
      <svg viewBox="0 0 10 10" class="h-2.5 w-2.5" fill="none" stroke="currentColor" stroke-width="1">
        {#if button === 'min'}
          <path d="M0 5h10" />
        {:else if button === 'max'}
          <rect x="0.5" y="0.5" width="9" height="9" />
        {:else}
          <path d="M0 0l10 10M10 0 0 10" />
        {/if}
      </svg>
    </div>
  {/each}
</div>
