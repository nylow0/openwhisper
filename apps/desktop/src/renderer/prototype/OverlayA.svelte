<script lang="ts">
  // PROTOTYPE overlay A, "Ledger": a single black line. One coloured dot is the only signal.
  import { fade } from 'svelte/transition';
  import { formatElapsed, type OverlayPhase } from './mock';

  export let phase: OverlayPhase;
  export let elapsedMs: number;
  export let partial: string;
  export let finalText: string;
  export let errorText: string;
</script>

<div class="flex h-full items-end justify-center p-5" style="font-family: 'Plus Jakarta Sans', sans-serif;">
  <div class="flex w-full items-center gap-3 rounded-full border border-[#262626] bg-black py-3 pl-5 pr-5 shadow-[0_10px_40px_rgba(0,0,0,0.45)]">
    <span class="relative flex h-2 w-2 shrink-0">
      {#if phase === 'recording'}
        <span class="absolute inset-0 animate-ring-pulse rounded-full bg-[#ff5a4e]"></span>
      {/if}
      <span
        class="h-2 w-2 rounded-full {phase === 'recording'
          ? 'bg-[#ff5a4e]'
          : phase === 'transcribing'
            ? 'animate-breathe bg-white'
            : phase === 'done'
              ? 'bg-white'
              : 'bg-[#ff5a4e]'}"
      ></span>
    </span>
    {#key phase}
      <p class="min-w-0 flex-1 truncate text-[13px] text-[#a3a3a3]" in:fade={{ duration: 120 }}>
        {#if phase === 'recording'}
          <b class="font-semibold text-white">Listening</b>
          <span class="ml-1 tabular-nums">{formatElapsed(elapsedMs)}</span>
        {:else if phase === 'transcribing'}
          <b class="font-semibold text-white">Writing.</b> {partial}…
        {:else if phase === 'done'}
          <b class="font-semibold text-white">Typed.</b> {finalText}
        {:else}
          <b class="font-semibold text-white">Needs attention.</b> {errorText}
        {/if}
      </p>
    {/key}
    {#if phase === 'recording'}
      <span class="shrink-0 text-[12px] text-[#5c5c5c]">Release to type</span>
    {/if}
  </div>
</div>
