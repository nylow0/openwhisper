<script lang="ts">
  // PROTOTYPE overlay B, "Console": a slim terminal strip with a level meter and braille spinner.
  import { formatElapsed, wordCount, type OverlayPhase } from './mock';

  export let phase: OverlayPhase;
  export let elapsedMs: number;
  export let partial: string;
  export let finalText: string;
  export let errorText: string;

  const AMBER = '#ffb224';
  const SPINNER = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
  const BARS = Array.from({ length: 18 }, (_, index) => `${(index * 137) % 700}ms`);
</script>

<div class="flex h-full items-end justify-center p-5" style="font-family: 'Geist Mono', monospace;">
  <div class="flex h-11 w-full items-center gap-3 rounded-md border border-[#262626] bg-black px-3.5 text-[12px] shadow-[0_10px_40px_rgba(0,0,0,0.5)]">
    {#if phase === 'recording'}
      <span class="flex items-center gap-1.5" style="color: {AMBER}">
        <span class="h-1.5 w-1.5 animate-breathe rounded-full" style="background: {AMBER}"></span>rec
      </span>
      <span class="tabular-nums text-white">{formatElapsed(elapsedMs)}</span>
      <span class="flex h-4 flex-1 items-center gap-[2px]">
        {#each BARS as delay}
          <span class="h-full w-[3px] origin-center animate-waveform rounded-[1px]" style="background: {AMBER}; animation-delay: {delay}"></span>
        {/each}
      </span>
      <span class="text-[#4d4d4d]">release ↑ to type</span>
    {:else if phase === 'transcribing'}
      <span style="color: {AMBER}">{SPINNER[Math.floor(elapsedMs / 100) % SPINNER.length]}</span>
      <span class="text-[#8a8a8a]">decoding</span>
      <span class="min-w-0 flex-1 truncate text-white" style="font-family: Geist, sans-serif;">{partial}<span class="animate-breathe">▍</span></span>
    {:else if phase === 'done'}
      <span class="text-[#4ade80]">✓</span>
      <span class="shrink-0 text-[#8a8a8a]">typed {wordCount(finalText)}w</span>
      <span class="min-w-0 flex-1 truncate text-[#bdbdbd]" style="font-family: Geist, sans-serif;">{finalText}</span>
    {:else}
      <span class="rounded-sm bg-[#ff6b6b] px-1 text-black">ERR</span>
      <span class="min-w-0 flex-1 truncate text-[#ff6b6b]">{errorText}</span>
    {/if}
  </div>
</div>
