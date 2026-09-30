<script lang="ts">
  // PROTOTYPE overlay C, "Stage": a caption card. Your words, set large, as you speak them.
  import { fade } from 'svelte/transition';
  import { formatElapsed, type OverlayPhase } from './mock';

  export let phase: OverlayPhase;
  export let elapsedMs: number;
  export let partial: string;
  export let finalText: string;
  export let errorText: string;

  const LILAC = '#b9a6ff';
  const SERIF = "'Instrument Serif', serif";
  const DOTS = Array.from({ length: 28 }, (_, index) => `${(index * 90) % 850}ms`);
</script>

<div class="flex h-full items-end justify-center p-4" style="font-family: Geist, sans-serif;">
  <div class="w-full rounded-[22px] border border-[#1f1f1f] bg-black/95 px-6 py-4 text-center shadow-[0_16px_50px_rgba(0,0,0,0.5)]">
    {#key phase}
      <div in:fade={{ duration: 140 }}>
        {#if phase === 'recording'}
          <p class="text-[24px] italic leading-tight text-white" style="font-family: {SERIF}">Listening…</p>
          <div class="mt-2.5 flex h-4 items-center justify-center gap-[3px]">
            {#each DOTS as delay}
              <span class="h-full w-[3px] animate-waveform rounded-full" style="background: {LILAC}; animation-delay: {delay}; opacity: 0.85"></span>
            {/each}
          </div>
          <p class="mt-2 text-[11px] tabular-nums text-[#5c5c5c]">{formatElapsed(elapsedMs)} · let go of Ctrl + Win to type</p>
        {:else if phase === 'transcribing'}
          <p class="line-clamp-2 text-[22px] leading-tight text-[#a3a3a3]" style="font-family: {SERIF}">
            {partial}<span class="ml-0.5 inline-block h-5 w-[2px] translate-y-1 animate-breathe" style="background: {LILAC}"></span>
          </p>
        {:else if phase === 'done'}
          <p class="line-clamp-2 text-[22px] leading-tight text-white" style="font-family: {SERIF}">{finalText}</p>
          <p class="mt-1.5 text-[11px] font-medium uppercase tracking-[0.14em]" style="color: {LILAC}">Typed</p>
        {:else}
          <p class="text-[22px] leading-tight text-[#ff7a8a]" style="font-family: {SERIF}">Can't hear you.</p>
          <p class="mt-1 line-clamp-1 text-[12px] text-[#8a8a8a]">{errorText}</p>
        {/if}
      </div>
    {/key}
  </div>
</div>
