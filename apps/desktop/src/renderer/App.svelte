<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { flip } from 'svelte/animate';
  import type { ErrorEvent, TranscriptFinalEvent } from '../shared/types';

  type TranscriptItem = {
    id: number;
    text: string;
    language: string | null;
    latencyMs: number | null;
    createdAt: string;
  };

  type Toast = {
    id: number;
    message: string;
    createdAt: string;
  };

  const api = window.api;

  let transcripts: TranscriptItem[] = [];
  let toasts: Toast[] = [];
  let nextTranscriptId = 1;
  let nextToastId = 1;
  let copiedId: number | null = null;
  let copiedResetTimer: ReturnType<typeof setTimeout> | undefined;

  let unsubTranscriptFinal: (() => void) | undefined;
  let unsubError: (() => void) | undefined;
  let unsubDisconnected: (() => void) | undefined;

  onMount(() => {
    if (!api) {
      pushToast('Electron preload API is unavailable. Rebuild and restart the app.');
      return;
    }

    unsubTranscriptFinal = api.onTranscriptFinal((data: TranscriptFinalEvent) => {
      const text = data.text.trim();
      if (!text) return;
      transcripts = [
        {
          id: nextTranscriptId,
          text,
          language: data.language ?? null,
          latencyMs: data.processingLatencyMs ?? null,
          createdAt: shortTime(),
        },
        ...transcripts,
      ];
      nextTranscriptId += 1;
    });

    unsubError = api.onError((data: ErrorEvent) => {
      pushToast(`[${data.code}] ${data.message}`);
    });

    unsubDisconnected = api.onDisconnected(() => {
      pushToast('Lost connection to the native bridge.');
    });
  });

  onDestroy(() => {
    unsubTranscriptFinal?.();
    unsubError?.();
    unsubDisconnected?.();
    if (copiedResetTimer) clearTimeout(copiedResetTimer);
  });

  async function copyTranscript(item: TranscriptItem) {
    try {
      await navigator.clipboard.writeText(item.text);
      copiedId = item.id;
      if (copiedResetTimer) clearTimeout(copiedResetTimer);
      copiedResetTimer = setTimeout(() => {
        copiedId = null;
      }, 1500);
    } catch (e) {
      pushToast('Failed to copy transcript: ' + (e as Error).message);
    }
  }

  function clearTranscripts() {
    transcripts = [];
  }

  function pushToast(message: string) {
    const id = nextToastId;
    nextToastId += 1;
    toasts = [{ id, message, createdAt: shortTime() }, ...toasts].slice(0, 4);
    setTimeout(() => dismissToast(id), 6000);
  }

  function dismissToast(id: number) {
    toasts = toasts.filter((toast) => toast.id !== id);
  }

  function shortTime() {
    return new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }
</script>

<div class="flex h-screen flex-col overflow-hidden bg-zinc-950 text-zinc-100">
  <!-- Title bar — draggable, integrated, native window controls overlaid at right -->
  <header
    class="drag flex h-11 shrink-0 select-none items-center gap-2.5 border-b border-zinc-800/80 bg-zinc-950 px-4"
  >
    <div class="flex h-6 w-6 items-center justify-center rounded-md bg-indigo-500/15 text-indigo-400">
      <svg viewBox="0 0 24 24" fill="none" class="h-3.5 w-3.5" aria-hidden="true">
        <circle cx="12" cy="12" r="2.4" fill="currentColor" />
        <path d="M7.5 7.5a6.4 6.4 0 0 0 0 9" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
        <path d="M16.5 7.5a6.4 6.4 0 0 1 0 9" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
        <path d="M4.4 4.4a10.6 10.6 0 0 0 0 15.2" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" opacity="0.45" />
        <path d="M19.6 4.4a10.6 10.6 0 0 1 0 15.2" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" opacity="0.45" />
      </svg>
    </div>
    <span class="text-[13px] font-semibold tracking-tight text-zinc-100">OpenWhisper</span>
    <span class="text-zinc-700">·</span>
    <span class="text-[12px] font-medium text-zinc-500">Offline dictation</span>
  </header>

  <!-- Main content -->
  <main class="mx-auto flex w-full max-w-2xl min-h-0 flex-1 flex-col px-6">
    <!-- How-to-dictate hint -->
    <section
      class="mt-6 flex shrink-0 items-center gap-3.5 rounded-2xl border border-zinc-800 bg-gradient-to-br from-zinc-900 to-zinc-900/30 p-4"
    >
      <div class="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl bg-indigo-500/15 text-indigo-400">
        <svg viewBox="0 0 24 24" fill="none" class="h-5 w-5" aria-hidden="true">
          <circle cx="12" cy="12" r="2.6" fill="currentColor" />
          <path d="M7.5 7.5a6.4 6.4 0 0 0 0 9" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
          <path d="M16.5 7.5a6.4 6.4 0 0 1 0 9" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
          <path d="M4.4 4.4a10.6 10.6 0 0 0 0 15.2" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" opacity="0.45" />
          <path d="M19.6 4.4a10.6 10.6 0 0 1 0 15.2" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" opacity="0.45" />
        </svg>
      </div>
      <div class="min-w-0">
        <div class="flex items-center gap-1.5">
          <span class="text-[13px] font-semibold text-zinc-100">Hold</span>
          <kbd class="rounded-md border border-zinc-700 bg-zinc-800 px-1.5 py-0.5 text-[10px] font-semibold text-zinc-300">Ctrl</kbd>
          <span class="text-[10px] text-zinc-600">+</span>
          <kbd class="rounded-md border border-zinc-700 bg-zinc-800 px-1.5 py-0.5 text-[10px] font-semibold text-zinc-300">Win</kbd>
          <span class="text-[13px] font-semibold text-zinc-100">to dictate</span>
        </div>
        <p class="mt-1 text-[12px] leading-snug text-zinc-500">
          Speak while holding the keys, then release — your words are typed straight into whatever
          app you're using.
        </p>
      </div>
    </section>

    <!-- History header -->
    <div class="flex shrink-0 items-center justify-between gap-3 pb-2.5 pt-6">
      <div class="flex items-center gap-2">
        <h2 class="text-[13px] font-semibold text-zinc-300">History</h2>
        {#if transcripts.length > 0}
          <span class="rounded-full bg-zinc-800 px-1.5 py-0.5 text-[10px] font-semibold text-zinc-400">
            {transcripts.length}
          </span>
        {/if}
      </div>
      <button
        type="button"
        class="rounded-lg px-2 py-1 text-[12px] font-medium text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200 disabled:pointer-events-none disabled:opacity-40"
        on:click={clearTranscripts}
        disabled={transcripts.length === 0}
      >
        Clear
      </button>
    </div>

    <!-- Transcript list -->
    <div class="scroll-area min-h-0 flex-1 space-y-2 overflow-y-auto pb-5 pr-1">
      {#if transcripts.length === 0}
        <div class="flex min-h-[220px] flex-col items-center justify-center gap-3 text-center">
          <div class="flex h-14 w-14 items-center justify-center rounded-2xl border border-zinc-800 bg-zinc-900 text-zinc-700">
            <svg viewBox="0 0 24 24" fill="none" class="h-6 w-6" aria-hidden="true">
              <circle cx="12" cy="12" r="2.4" fill="currentColor" />
              <path d="M7.5 7.5a6.4 6.4 0 0 0 0 9" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
              <path d="M16.5 7.5a6.4 6.4 0 0 1 0 9" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
              <path d="M4.4 4.4a10.6 10.6 0 0 0 0 15.2" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" opacity="0.45" />
              <path d="M19.6 4.4a10.6 10.6 0 0 1 0 15.2" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" opacity="0.45" />
            </svg>
          </div>
          <div>
            <p class="text-[13px] font-medium text-zinc-400">No transcripts yet</p>
            <p class="mt-0.5 text-[12px] text-zinc-600">
              Dictate something with Ctrl + Win and it will appear here.
            </p>
          </div>
        </div>
      {/if}

      {#each transcripts as transcript (transcript.id)}
        <article
          in:fly={{ y: 8, duration: 220 }}
          animate:flip={{ duration: 220 }}
          class="group rounded-xl border border-zinc-800 bg-zinc-900 p-3.5 transition-colors hover:border-zinc-700"
        >
          <div class="mb-1.5 flex items-center justify-between gap-2">
            <div class="flex items-center gap-2 text-[11px] text-zinc-500">
              <span class="tabular-nums">{transcript.createdAt}</span>
              {#if transcript.language}
                <span class="rounded bg-zinc-800 px-1.5 py-0.5 font-medium uppercase text-zinc-400">
                  {transcript.language}
                </span>
              {/if}
              {#if transcript.latencyMs !== null}
                <span class="tabular-nums">{transcript.latencyMs} ms</span>
              {/if}
            </div>
            <button
              type="button"
              class="inline-flex shrink-0 items-center gap-1 rounded-md px-1.5 py-1 text-[11px] font-medium opacity-60 transition-all hover:bg-zinc-800 group-hover:opacity-100 {copiedId ===
              transcript.id
                ? 'text-emerald-400'
                : 'text-zinc-400 hover:text-zinc-100'}"
              on:click={() => copyTranscript(transcript)}
            >
              {#if copiedId === transcript.id}
                <svg
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2.5"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  class="h-3.5 w-3.5"
                  aria-hidden="true"
                >
                  <path d="M20 6 9 17l-5-5" />
                </svg>
                Copied
              {:else}
                <svg
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  class="h-3.5 w-3.5"
                  aria-hidden="true"
                >
                  <rect x="9" y="9" width="11" height="11" rx="2.5" />
                  <path d="M5 15V5.5A2.5 2.5 0 0 1 7.5 3H16" />
                </svg>
                Copy
              {/if}
            </button>
          </div>
          <p class="whitespace-pre-wrap break-words text-[14px] leading-relaxed text-zinc-100">
            {transcript.text}
          </p>
        </article>
      {/each}
    </div>
  </main>

  <!-- Toast notifications -->
  <div class="pointer-events-none fixed bottom-4 right-4 z-50 flex w-80 flex-col gap-2">
    {#each toasts as toast (toast.id)}
      <div
        in:fly={{ x: 20, duration: 200 }}
        out:fade={{ duration: 150 }}
        class="pointer-events-auto flex items-start gap-2.5 rounded-xl border border-rose-500/30 bg-zinc-900 p-3 shadow-xl shadow-black/50"
      >
        <span class="mt-px shrink-0 text-rose-400">
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="h-4 w-4"
            aria-hidden="true"
          >
            <path d="M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z" />
            <line x1="12" y1="9" x2="12" y2="13" />
            <line x1="12" y1="17" x2="12.01" y2="17" />
          </svg>
        </span>
        <div class="min-w-0 flex-1">
          <p class="text-[12px] font-medium leading-snug text-zinc-200">{toast.message}</p>
          <p class="mt-0.5 text-[10px] tabular-nums text-zinc-500">{toast.createdAt}</p>
        </div>
        <button
          type="button"
          class="shrink-0 rounded text-zinc-600 transition-colors hover:text-zinc-300"
          on:click={() => dismissToast(toast.id)}
          aria-label="Dismiss"
        >
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.2"
            stroke-linecap="round"
            class="h-3.5 w-3.5"
            aria-hidden="true"
          >
            <path d="M18 6 6 18M6 6l12 12" />
          </svg>
        </button>
      </div>
    {/each}
  </div>
</div>
