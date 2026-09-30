<script lang="ts">
  // PROTOTYPE overlay D, "Split": a compact status card with a segmented level meter.
  import { formatElapsed, type OverlayPhase } from './mock';

  export let phase: OverlayPhase;
  export let elapsedMs: number;
  export let partial: string;
  export let finalText: string;
  export let errorText: string;

  const BLUE = '#4c8dff';
  const CELLS = 32;

  // Fake mic level so the meter moves; the real one would come from the ASR worker.
  $: level = 0.35 + 0.3 * Math.sin(elapsedMs / 260) + ((elapsedMs * 7919) % 100) / 500;
</script>

<div class="flex h-full items-end justify-center p-5" style="font-family: Geist, sans-serif;">
  <div class="w-full rounded-xl border border-[#1f1f1f] bg-[#0a0a0a] px-4 py-3 shadow-[0_12px_40px_rgba(0,0,0,0.5)]">
    <div class="flex items-center gap-2.5">
      {#if phase === 'recording'}
        <span class="h-2 w-2 animate-breathe rounded-full bg-[#ff5a5a]"></span>
        <span class="text-[13px] font-medium text-white">Recording</span>
        <span class="text-[12px] tabular-nums text-[#6b6b6b]">{formatElapsed(elapsedMs)}</span>
      {:else if phase === 'transcribing'}
        <span class="h-2 w-2 rounded-full" style="background: {BLUE}"></span>
        <span class="text-[13px] font-medium text-white">Transcribing</span>
      {:else if phase === 'done'}
        <svg viewBox="0 0 24 24" class="h-3.5 w-3.5" fill="none" stroke={BLUE} stroke-width="3" stroke-linecap="round"><path d="M5 12l5 5 9-10" /></svg>
        <span class="text-[13px] font-medium text-white">Typed</span>
      {:else}
        <span class="h-2 w-2 rounded-full bg-[#ff6b6b]"></span>
        <span class="text-[13px] font-medium text-[#ff6b6b]">Dictation failed</span>
      {/if}
      <span class="ml-auto flex gap-1">
        {#each ['Ctrl', 'Win'] as key}
          <kbd class="rounded border border-[#262626] border-b-[#333] bg-black px-1.5 py-px font-sans text-[10px] text-[#6b6b6b]">{key}</kbd>
        {/each}
      </span>
    </div>

    {#if phase === 'recording'}
      <div class="mt-2.5 flex h-1.5 gap-[3px]">
        {#each Array(CELLS) as _, index}
          <span class="flex-1 rounded-[1px] transition-colors duration-100" style="background: {index / CELLS < level ? BLUE : '#1c1c1c'}"></span>
        {/each}
      </div>
    {:else if phase === 'transcribing'}
      <div class="relative mt-2.5 h-[3px] overflow-hidden rounded-full bg-[#1c1c1c]">
        <span class="sweep absolute inset-y-0 w-1/3 rounded-full" style="background: {BLUE}"></span>
      </div>
      <p class="mt-2 truncate text-[12px] text-[#8a8a8a]">{partial}…</p>
    {:else if phase === 'done'}
      <p class="mt-1.5 line-clamp-2 text-[12px] leading-snug text-[#bdbdbd]">{finalText}</p>
    {:else}
      <p class="mt-1.5 line-clamp-2 text-[12px] leading-snug text-[#8a8a8a]">{errorText}</p>
    {/if}
  </div>
</div>

<style>
  .sweep {
    animation: sweep 1.1s ease-in-out infinite;
  }

  @keyframes sweep {
    from {
      left: -33%;
    }
    to {
      left: 100%;
    }
  }
</style>
