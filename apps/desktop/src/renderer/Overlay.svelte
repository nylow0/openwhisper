<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import type { ErrorEvent, TranscriptFinalEvent, TranscriptPartialEvent } from '../shared/types';

  type Phase = 'recording' | 'transcribing' | 'done' | 'error';

  const api = window.api;
  // Staggered delays give the equalizer bars a lively, irregular motion.
  const BAR_DELAYS = ['0ms', '140ms', '280ms', '120ms', '360ms', '220ms', '60ms'];

  let phase: Phase = 'recording';
  let partial = '';
  let finalText = '';
  let errorText = '';
  let elapsedMs = 0;

  let startedAt = 0;
  let timer: ReturnType<typeof setInterval> | undefined;
  let unsubs: Array<() => void> = [];

  $: title =
    phase === 'recording'
      ? 'Listening'
      : phase === 'transcribing'
        ? 'Transcribing'
        : phase === 'done'
          ? 'Inserted'
          : 'Dictation failed';

  $: detail =
    phase === 'recording'
      ? 'Release Ctrl + Win to insert'
      : phase === 'transcribing'
        ? partial || 'Converting your speech to text…'
        : phase === 'done'
          ? finalText || 'No speech detected'
          : errorText;

  $: borderClass = phase === 'recording' ? 'border-indigo-500/40' : 'border-zinc-700/70';

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

<div class="flex h-screen items-center justify-center p-5">
  <div
    class="flex w-full items-center gap-3.5 rounded-2xl border bg-zinc-900/95 px-4 py-3 shadow-2xl shadow-black/70 backdrop-blur-md {borderClass}"
  >
    <!-- State icon / live waveform -->
    <div class="flex h-10 w-10 shrink-0 items-center justify-center">
      {#if phase === 'recording'}
        <div class="flex h-7 items-end gap-[3px]">
          {#each BAR_DELAYS as delay}
            <span
              class="w-[3px] origin-bottom animate-waveform rounded-full bg-indigo-400"
              style="height: 100%; animation-delay: {delay}"
            ></span>
          {/each}
        </div>
      {:else if phase === 'transcribing'}
        <svg viewBox="0 0 24 24" fill="none" class="h-6 w-6 animate-spin text-indigo-400" aria-hidden="true">
          <circle cx="12" cy="12" r="9" stroke="currentColor" stroke-width="2.5" class="opacity-20" />
          <path d="M21 12a9 9 0 0 0-9-9" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" />
        </svg>
      {:else if phase === 'done'}
        <div class="flex h-9 w-9 items-center justify-center rounded-full bg-emerald-500/15 text-emerald-400">
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.5"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="h-5 w-5"
            aria-hidden="true"
          >
            <path d="M20 6 9 17l-5-5" />
          </svg>
        </div>
      {:else}
        <div class="flex h-9 w-9 items-center justify-center rounded-full bg-rose-500/15 text-rose-400">
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="h-5 w-5"
            aria-hidden="true"
          >
            <path d="M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z" />
            <line x1="12" y1="9" x2="12" y2="13" />
            <line x1="12" y1="17" x2="12.01" y2="17" />
          </svg>
        </div>
      {/if}
    </div>

    <!-- Status text -->
    {#key phase}
      <div class="min-w-0 flex-1" in:fade={{ duration: 140 }}>
        <div class="flex items-baseline gap-2">
          <p class="text-[13px] font-semibold text-zinc-100">{title}</p>
          {#if phase === 'recording'}
            <span class="font-mono text-[11px] tabular-nums text-indigo-300">{formatElapsed(elapsedMs)}</span>
          {/if}
        </div>
        <p class="mt-0.5 line-clamp-2 text-[12px] leading-snug text-zinc-400">{detail}</p>
      </div>
    {/key}

    <!-- Hotkey hint -->
    <div class="flex shrink-0 items-center gap-1">
      <kbd class="rounded-md border border-zinc-700 bg-zinc-800 px-1.5 py-0.5 text-[10px] font-semibold text-zinc-400">
        Ctrl
      </kbd>
      <span class="text-[10px] text-zinc-600">+</span>
      <kbd class="rounded-md border border-zinc-700 bg-zinc-800 px-1.5 py-0.5 text-[10px] font-semibold text-zinc-400">
        Win
      </kbd>
    </div>
  </div>
</div>
