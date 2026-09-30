<script lang="ts">
  // PROTOTYPE overlay E: D's status card in A's voice, with a blue→cyan level meter.
  import { formatElapsed, type OverlayPhase } from './mock';

  export let phase: OverlayPhase;
  export let elapsedMs: number;
  export let partial: string;
  export let finalText: string;
  export let errorText: string;

  const CELLS = 30;

  // Fake mic level so the meter moves; the real one would come from the ASR worker.
  $: level = 0.35 + 0.3 * Math.sin(elapsedMs / 260) + ((elapsedMs * 7919) % 100) / 500;

  function cellColor(index: number): string {
    // Lit cells blend from blue (quiet) to cyan (loud).
    const t = index / (CELLS - 1);
    const r = Math.round(59 + (34 - 59) * t);
    const g = Math.round(130 + (211 - 130) * t);
    const b = Math.round(246 + (238 - 246) * t);
    return `rgb(${r}, ${g}, ${b})`;
  }
</script>

<div class="flex h-full items-end justify-center p-5" style="font-family: 'Plus Jakarta Sans', sans-serif;">
  <div class="w-full rounded-2xl border border-[#1b2027] bg-[#0f1216] px-4 py-3 shadow-[0_12px_40px_rgba(0,0,0,0.5)]">
    <div class="flex items-center gap-2.5 text-[13px]">
      {#if phase === 'recording'}
        <span class="h-2 w-2 animate-breathe rounded-full bg-[#22d3ee] shadow-[0_0_8px_#22d3ee]"></span>
        <b class="font-semibold text-white">Listening</b>
        <span class="tabular-nums text-[#5d6773]">{formatElapsed(elapsedMs)}</span>
        <span class="ml-auto text-[12px] text-[#4a535e]">Release to type</span>
      {:else if phase === 'transcribing'}
        <span class="h-2 w-2 animate-breathe rounded-full bg-[#3b82f6]"></span>
        <b class="font-semibold text-white">Writing.</b>
        <span class="min-w-0 flex-1 truncate text-[#9aa4b0]">{partial}…</span>
      {:else if phase === 'done'}
        <span class="h-2 w-2 rounded-full bg-[#22d3ee]"></span>
        <b class="font-semibold text-white">Typed.</b>
        <span class="min-w-0 flex-1 truncate text-[#9aa4b0]">{finalText}</span>
      {:else}
        <span class="h-2 w-2 rounded-full bg-[#fb7185]"></span>
        <b class="font-semibold text-white">Needs attention.</b>
        <span class="min-w-0 flex-1 truncate text-[#9aa4b0]">{errorText}</span>
      {/if}
    </div>

    {#if phase === 'recording'}
      <div class="mt-2.5 flex h-1.5 gap-[3px]">
        {#each Array(CELLS) as _, index}
          <span class="flex-1 rounded-[1px] transition-colors duration-100" style="background: {index / CELLS < level ? cellColor(index) : '#1b2027'}"></span>
        {/each}
      </div>
    {:else if phase === 'transcribing'}
      <div class="relative mt-2.5 h-[3px] overflow-hidden rounded-full bg-[#1b2027]">
        <span class="sweep absolute inset-y-0 w-1/3 rounded-full"></span>
      </div>
    {/if}
  </div>
</div>

<style>
  .sweep {
    background: linear-gradient(90deg, #3b82f6, #22d3ee);
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
