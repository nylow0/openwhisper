<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import type {
    DictationStartedEvent,
    DictationStoppedEvent,
    ErrorEvent,
    StatusEvent,
    TranscriptFinalEvent,
    TranscriptPartialEvent,
  } from '../shared/types';

  type TranscriptItem = {
    id: number;
    text: string;
    language: string | null;
    latencyMs: number | null;
    createdAt: string;
  };

  type ErrorItem = {
    id: number;
    message: string;
    createdAt: string;
  };

  const api = window.api;

  let connectionStatus = 'Starting';
  let bridgeReady = false;
  let isDictating = false;
  let isTranscribing = false;
  let isModelLoaded = false;
  let workerHealthy = false;
  let currentPartial = '';
  let statusMessage = 'Ready when the bridge connects.';
  let transcripts: TranscriptItem[] = [];
  let errors: ErrorItem[] = [];
  let nextTranscriptId = 1;
  let nextErrorId = 1;

  $: dictationPhase = isTranscribing ? 'Transcribing' : isDictating ? 'Listening' : 'Idle';
  $: primaryAction = isDictating ? 'Stop Dictation' : isTranscribing ? 'Transcribing' : 'Start Dictation';
  $: canStart = Boolean(api) && bridgeReady && !isDictating && !isTranscribing;
  $: canStop = Boolean(api) && bridgeReady && isDictating && !isTranscribing;
  $: latestTranscript = transcripts[0]?.text ?? '';
  $: connectionTone = bridgeReady && workerHealthy ? 'Ready' : bridgeReady ? 'Degraded' : 'Offline';

  let unsubTranscriptPartial: (() => void) | undefined;
  let unsubTranscriptFinal: (() => void) | undefined;
  let unsubDictationStarted: (() => void) | undefined;
  let unsubDictationStopped: (() => void) | undefined;
  let unsubStatus: (() => void) | undefined;
  let unsubError: (() => void) | undefined;
  let unsubDisconnected: (() => void) | undefined;
  let unsubHotkeyToggle: (() => void) | undefined;

  onMount(async () => {
    if (!api) {
      connectionStatus = 'Bridge unavailable';
      statusMessage = 'Electron preload did not expose the native bridge.';
      pushError('Electron preload API is unavailable. Rebuild and restart the packaged app.');
      return;
    }

    unsubTranscriptPartial = api.onTranscriptPartial((data: TranscriptPartialEvent) => {
      currentPartial = data.text;
      statusMessage = 'Listening...';
    });

    unsubTranscriptFinal = api.onTranscriptFinal((data: TranscriptFinalEvent) => {
      const text = data.text.trim();
      currentPartial = '';
      isTranscribing = false;
      statusMessage = text ? 'Transcript ready.' : 'No speech was detected.';

      if (text) {
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
      }
    });

    unsubDictationStarted = api.onDictationStarted((data: DictationStartedEvent) => {
      isDictating = true;
      isTranscribing = false;
      statusMessage = `Recording started at ${formatUnixSeconds(data.timestamp)}.`;
    });

    unsubDictationStopped = api.onDictationStopped((data: DictationStoppedEvent) => {
      isDictating = false;
      isTranscribing = true;
      statusMessage = `Recording stopped at ${formatUnixSeconds(data.timestamp)}.`;
    });

    unsubStatus = api.onStatusUpdate((data: StatusEvent) => {
      applyStatus(data);
    });

    unsubError = api.onError((data: ErrorEvent) => {
      isDictating = false;
      isTranscribing = false;
      pushError(`[${data.code}] ${data.message}`);
    });

    unsubDisconnected = api.onDisconnected(() => {
      bridgeReady = false;
      workerHealthy = false;
      isDictating = false;
      isTranscribing = false;
      connectionStatus = 'Disconnected';
      statusMessage = 'Native bridge disconnected.';
    });

    unsubHotkeyToggle = api.onHotkeyToggle(() => {
      void toggleDictation();
    });

    await loadInitialStatus();
  });

  onDestroy(() => {
    unsubTranscriptPartial?.();
    unsubTranscriptFinal?.();
    unsubDictationStarted?.();
    unsubDictationStopped?.();
    unsubStatus?.();
    unsubError?.();
    unsubDisconnected?.();
    unsubHotkeyToggle?.();
  });

  async function startDictation() {
    if (!api || !canStart) return;
    try {
      currentPartial = '';
      isTranscribing = false;
      statusMessage = 'Starting recording...';
      await api.startDictation();
      isDictating = true;
    } catch (e) {
      pushError('Failed to start dictation: ' + (e as Error).message);
    }
  }

  async function stopDictation() {
    if (!api || !canStop) return;
    try {
      statusMessage = 'Stopping recording...';
      await api.stopDictation();
      isDictating = false;
      isTranscribing = true;
      currentPartial = '';
    } catch (e) {
      isTranscribing = false;
      pushError('Failed to stop dictation: ' + (e as Error).message);
    }
  }

  async function toggleDictation() {
    if (isTranscribing) return;
    if (isDictating) {
      await stopDictation();
      return;
    }
    await startDictation();
  }

  async function copyLatestTranscript() {
    if (!latestTranscript) return;
    try {
      await navigator.clipboard.writeText(latestTranscript);
      statusMessage = 'Latest transcript copied.';
    } catch (e) {
      pushError('Failed to copy transcript: ' + (e as Error).message);
    }
  }

  function clearTranscripts() {
    transcripts = [];
    currentPartial = '';
    statusMessage = 'Transcript history cleared.';
  }

  function clearErrors() {
    errors = [];
  }

  async function loadInitialStatus() {
    if (!api) return;

    for (let attempt = 1; attempt <= 10; attempt += 1) {
      try {
        const status = await api.getStatus();
        applyStatus(status);
        statusMessage = 'Bridge connected.';
        return;
      } catch (e) {
        if (attempt === 10) {
          connectionStatus = 'Connection error';
          pushError('Failed to get native status: ' + (e as Error).message);
          return;
        }
        await sleep(500);
      }
    }
  }

  async function refreshHealth() {
    if (!api) return;
    try {
      const result = await api.healthCheck();
      bridgeReady = true;
      workerHealthy = result.workerHealthy;
      isDictating = result.isDictating;
      isModelLoaded = result.isModelLoaded;
      connectionStatus = result.workerHealthy ? 'Connected' : 'Degraded';
      statusMessage = `Health checked at ${new Date(result.timestamp).toLocaleTimeString()}.`;
    } catch (e) {
      connectionStatus = 'Connection error';
      pushError('Health check failed: ' + (e as Error).message);
    }
  }

  function applyStatus(status: StatusEvent) {
    bridgeReady = true;
    connectionStatus = status.workerHealthy ? 'Connected' : 'Degraded';
    isDictating = status.isDictating;
    isModelLoaded = status.isModelLoaded;
    workerHealthy = status.workerHealthy;
  }

  function pushError(message: string) {
    errors = [{ id: nextErrorId, message, createdAt: shortTime() }, ...errors];
    nextErrorId += 1;
    statusMessage = 'Action needs attention.';
  }

  function shortTime() {
    return new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  function formatUnixSeconds(seconds: number) {
    return new Date(seconds * 1000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  function sleep(ms: number) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
</script>

<main class="min-h-screen bg-zinc-50 text-zinc-950">
  <div class="mx-auto flex min-h-screen w-full max-w-6xl flex-col gap-5 px-6 py-6">
    <header class="flex flex-col gap-4 border-b border-zinc-200 pb-5 sm:flex-row sm:items-end sm:justify-between">
      <div>
        <h1 class="text-3xl font-semibold tracking-normal">OpenWhisper</h1>
        <p class="mt-1 text-sm text-zinc-600">Windows-first offline dictation</p>
      </div>
      <div class="grid grid-cols-2 gap-2 text-sm sm:grid-cols-4">
        <div class="min-w-28 rounded border border-zinc-200 bg-white px-3 py-2">
          <p class="text-xs font-medium text-zinc-500">Bridge</p>
          <p class="mt-1 font-semibold" class:text-emerald-700={bridgeReady} class:text-rose-700={!bridgeReady}>
            {connectionStatus}
          </p>
        </div>
        <div class="min-w-28 rounded border border-zinc-200 bg-white px-3 py-2">
          <p class="text-xs font-medium text-zinc-500">Worker</p>
          <p class="mt-1 font-semibold" class:text-emerald-700={workerHealthy} class:text-rose-700={!workerHealthy}>
            {connectionTone}
          </p>
        </div>
        <div class="min-w-28 rounded border border-zinc-200 bg-white px-3 py-2">
          <p class="text-xs font-medium text-zinc-500">Dictation</p>
          <p class="mt-1 font-semibold" class:text-emerald-700={isDictating} class:text-blue-700={isTranscribing}>
            {dictationPhase}
          </p>
        </div>
        <div class="min-w-28 rounded border border-zinc-200 bg-white px-3 py-2">
          <p class="text-xs font-medium text-zinc-500">Model</p>
          <p class="mt-1 font-semibold" class:text-emerald-700={isModelLoaded} class:text-zinc-500={!isModelLoaded}>
            {isModelLoaded ? 'Loaded' : 'Pending'}
          </p>
        </div>
      </div>
    </header>

    <section class="grid gap-5 lg:grid-cols-[360px_minmax(0,1fr)]">
      <div class="rounded border border-zinc-200 bg-white p-5">
        <div class="flex items-start justify-between gap-4">
          <div>
            <h2 class="text-lg font-semibold">Dictation</h2>
            <p class="mt-1 text-sm text-zinc-600">{statusMessage}</p>
          </div>
          <span
            class="mt-1 h-3 w-3 shrink-0 rounded-full"
            class:bg-emerald-500={isDictating}
            class:bg-blue-500={isTranscribing}
            class:bg-zinc-300={!isDictating && !isTranscribing}
          ></span>
        </div>

        <button
          class="mt-6 h-14 w-full rounded bg-zinc-950 px-5 text-base font-semibold text-white transition hover:bg-zinc-800 disabled:bg-zinc-300 disabled:text-zinc-600"
          on:click={isDictating ? stopDictation : startDictation}
          disabled={isDictating ? !canStop : !canStart}
        >
          {primaryAction}
        </button>

        <div class="mt-3 grid grid-cols-2 gap-3">
          <button
            class="h-10 rounded border border-zinc-300 bg-white px-3 text-sm font-medium text-zinc-800 hover:bg-zinc-100 disabled:text-zinc-400"
            on:click={refreshHealth}
            disabled={!api}
          >
            Refresh
          </button>
          <button
            class="h-10 rounded border border-zinc-300 bg-white px-3 text-sm font-medium text-zinc-800 hover:bg-zinc-100 disabled:text-zinc-400"
            on:click={copyLatestTranscript}
            disabled={!latestTranscript}
          >
            Copy Latest
          </button>
        </div>

        {#if currentPartial}
          <div class="mt-5 border-t border-zinc-200 pt-4">
            <p class="text-xs font-medium uppercase text-zinc-500">Live</p>
            <p class="mt-2 text-sm leading-6 text-zinc-700">{currentPartial}</p>
          </div>
        {/if}
      </div>

      <div class="rounded border border-zinc-200 bg-white p-5">
        <div class="flex items-center justify-between gap-3">
          <div>
            <h2 class="text-lg font-semibold">Transcript</h2>
            <p class="mt-1 text-sm text-zinc-600">
              {transcripts.length === 0 ? 'No transcript yet.' : `${transcripts.length} item${transcripts.length === 1 ? '' : 's'}`}
            </p>
          </div>
          <button
            class="h-9 rounded border border-zinc-300 bg-white px-3 text-sm font-medium text-zinc-800 hover:bg-zinc-100 disabled:text-zinc-400"
            on:click={clearTranscripts}
            disabled={transcripts.length === 0 && !currentPartial}
          >
            Clear
          </button>
        </div>

        <div class="mt-5 min-h-64">
          {#if isTranscribing}
            <p class="rounded border border-blue-200 bg-blue-50 px-3 py-2 text-sm text-blue-800">Transcribing audio...</p>
          {/if}

          {#if transcripts.length === 0 && !currentPartial && !isTranscribing}
            <div class="flex min-h-52 items-center justify-center rounded border border-dashed border-zinc-300 text-sm text-zinc-500">
              Start dictation to create a transcript.
            </div>
          {/if}

          {#if transcripts.length > 0}
            <div class="divide-y divide-zinc-200">
              {#each transcripts as transcript}
                <article class="py-4 first:pt-0 last:pb-0">
                  <div class="mb-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-zinc-500">
                    <span>{transcript.createdAt}</span>
                    {#if transcript.language}
                      <span>{transcript.language}</span>
                    {/if}
                    {#if transcript.latencyMs !== null}
                      <span>{transcript.latencyMs} ms</span>
                    {/if}
                  </div>
                  <p class="whitespace-pre-wrap text-base leading-7 text-zinc-900">{transcript.text}</p>
                </article>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    </section>

    {#if errors.length > 0}
      <section class="rounded border border-rose-200 bg-rose-50 p-5">
        <div class="flex items-center justify-between gap-3">
          <h2 class="text-lg font-semibold text-rose-800">Errors</h2>
          <button
            class="h-9 rounded border border-rose-300 bg-white px-3 text-sm font-medium text-rose-800 hover:bg-rose-100"
            on:click={clearErrors}
          >
            Clear
          </button>
        </div>
        <div class="mt-3 divide-y divide-rose-200">
          {#each errors as error}
            <p class="py-2 text-sm leading-6 text-rose-700">
              <span class="font-medium">{error.createdAt}</span>
              <span class="ml-2">{error.message}</span>
            </p>
          {/each}
        </div>
      </section>
    {/if}
  </div>
</main>
