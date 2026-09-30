<script lang="ts">
  // Recording overlay (HUD): a one-line pill near the bottom of the screen that follows
  // a dictation from recording to the typed result. One coloured dot carries the state:
  // pulsing cyan while listening, breathing blue while writing, rose on error.
  // The pill is pure black with a faint light border so it stays visible over dark apps.
  import { onDestroy, onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import type { ErrorEvent, TranscriptFinalEvent, TranscriptPartialEvent } from '../shared/types';

  type Phase = 'recording' | 'transcribing' | 'done' | 'error';

  const api = window.api;

  let phase: Phase = 'recording';
  let partial = '';
  let finalText = '';
  let errorText = '';
  let elapsedMs = 0;

  let startedAt = 0;
  let timer: ReturnType<typeof setInterval> | undefined;
  let unsubs: Array<() => void> = [];

  onMount(() => {
    if (!api) return;
    unsubs = [
      api.onDictationStarted(() => {
        phase = 'recording';
        partial = '';
        finalText = '';
        errorText = '';
        startTimer();
      }),
      api.onTranscriptPartial((data: TranscriptPartialEvent) => {
        partial = data.text;
      }),
      api.onDictationStopped(() => {
        phase = 'transcribing';
        stopTimer();
      }),
      api.onTranscriptFinal((data: TranscriptFinalEvent) => {
        phase = 'done';
        finalText = data.text.trim();
        stopTimer();
      }),
      api.onError((data: ErrorEvent) => {
        phase = 'error';
        errorText = data.message;
        stopTimer();
      }),
      api.onDisconnected(() => {
        phase = 'error';
        errorText = 'Lost connection to the speech engine.';
        stopTimer();
      }),
    ];
  });

  onDestroy(() => {
    unsubs.forEach((unsub) => unsub());
    stopTimer();
  });

  function startTimer() {
    stopTimer();
    startedAt = Date.now();
    elapsedMs = 0;
    timer = setInterval(() => {
      elapsedMs = Date.now() - startedAt;
    }, 200);
  }

  function stopTimer() {
    if (timer) {
      clearInterval(timer);
      timer = undefined;
    }
  }

  function formatElapsed(ms: number) {
    const total = Math.floor(ms / 1000);
    const minutes = Math.floor(total / 60);
    const seconds = (total % 60).toString().padStart(2, '0');
    return `${minutes}:${seconds}`;
  }
</script>

<div class="flex h-screen items-end justify-center p-5">
  <div
    class="flex w-full items-center gap-3 rounded-full border border-white/20 bg-black py-3 pl-5 pr-5 shadow-[0_10px_40px_rgba(0,0,0,0.45)]"
  >
    <span class="relative flex h-2 w-2 shrink-0">
      {#if phase === 'recording'}
        <span class="absolute inset-0 animate-ring-pulse rounded-full bg-cyan-400"></span>
      {/if}
      <span
        class="h-2 w-2 rounded-full {phase === 'recording' || phase === 'done'
          ? 'bg-cyan-400'
          : phase === 'transcribing'
            ? 'animate-breathe bg-blue-500'
            : 'bg-rose-400'}"
      ></span>
    </span>

    {#key phase}
      <p class="min-w-0 flex-1 truncate text-[13px] text-ink-200" in:fade={{ duration: 120 }}>
        {#if phase === 'recording'}
          <b class="font-semibold text-white">Listening</b>
          <span class="ml-1 tabular-nums">{formatElapsed(elapsedMs)}</span>
        {:else if phase === 'transcribing'}
          <b class="font-semibold text-white">Writing.</b>
          {partial ? `${partial}…` : 'Turning your speech into text…'}
        {:else if phase === 'done'}
          {#if finalText}
            <b class="font-semibold text-white">Typed.</b> {finalText}
          {:else}
            <b class="font-semibold text-white">Nothing heard.</b> No speech was detected.
          {/if}
        {:else}
          <b class="font-semibold text-white">Needs attention.</b> {errorText}
        {/if}
      </p>
    {/key}

    {#if phase === 'recording'}
      <span class="shrink-0 text-[12px] text-ink-500">Release Ctrl + Win to type</span>
    {/if}
  </div>
</div>
