<script lang="ts">
  // PROTOTYPE overlay F: A's single-line pill on near-black, with a blue→cyan level line.
  import { fade } from 'svelte/transition';
  import { formatElapsed, type OverlayPhase } from './mock';

  export let phase: OverlayPhase;
  export let elapsedMs: number;
  export let partial: string;
  export let finalText: string;
  export let errorText: string;

  // Fake mic level so the line breathes; the real one would come from the ASR worker.
  $: level = Math.min(1, 0.35 + 0.3 * Math.sin(elapsedMs / 260) + ((elapsedMs * 7919) % 100) / 400);
</script>

<div class="flex h-full items-end justify-center p-5" style="font-family: 'Plus Jakarta Sans', sans-serif;">
  <div class="relative w-full overflow-hidden rounded-full border border-[#1b2027] bg-[#0f1216] py-3 pl-5 pr-5 shadow-[0_10px_40px_rgba(0,0,0,0.45)]">
    <div class="flex items-center gap-3">
      <span class="relative flex h-2 w-2 shrink-0">
        {#if phase === 'recording'}<span class="absolute inset-0 animate-ring-pulse rounded-full bg-[#22d3ee]"></span>{/if}
        <span
          class="h-2 w-2 rounded-full {phase === 'recording'
            ? 'bg-[#22d3ee]'
            : phase === 'transcribing'
              ? 'animate-breathe bg-[#3b82f6]'
              : phase === 'done'
                ? 'bg-[#22d3ee]'
                : 'bg-[#fb7185]'}"
        ></span>
      </span>
      {#key phase}
        <p class="min-w-0 flex-1 truncate text-[13px] text-[#9aa4b0]" in:fade={{ duration: 120 }}>
          {#if phase === 'recording'}
            <b class="font-semibold text-white">Listening</b> <span class="ml-1 tabular-nums">{formatElapsed(elapsedMs)}</span>
          {:else if phase === 'transcribing'}
            <b class="font-semibold text-white">Writing.</b> {partial}…
          {:else if phase === 'done'}
            <b class="font-semibold text-white">Typed.</b> {finalText}
          {:else}
            <b class="font-semibold text-white">Needs attention.</b> {errorText}
          {/if}
        </p>
      {/key}
      {#if phase === 'recording'}<span class="shrink-0 text-[12px] text-[#4a535e]">Release to type</span>{/if}
    </div>
    {#if phase === 'recording'}
      <span
        class="absolute bottom-0 left-1/2 h-[2px] -translate-x-1/2 rounded-full transition-[width] duration-150"
        style="width: {Math.round(level * 100)}%; background: linear-gradient(90deg, transparent, #3b82f6, #22d3ee, #3b82f6, transparent);"
      ></span>
    {/if}
  </div>
</div>
