<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { ErrorEvent, StatusEvent, TranscriptFinalEvent, TranscriptPartialEvent } from '../shared/types';

  let connectionStatus = 'Connecting to Rust...';
  let isDictating = false;
  let isModelLoaded = false;
  let workerHealthy = false;
  let currentPartial = '';
  let transcripts: string[] = [];
  let errors: string[] = [];

  let unsubTranscriptPartial: (() => void) | undefined;
  let unsubTranscriptFinal: (() => void) | undefined;
  let unsubStatus: (() => void) | undefined;
  let unsubError: (() => void) | undefined;
  let unsubDisconnected: (() => void) | undefined;

  onMount(async () => {
    // Set up event listeners
    unsubTranscriptPartial = window.api.onTranscriptPartial((data: TranscriptPartialEvent) => {
      currentPartial = data.text;
    });

    unsubTranscriptFinal = window.api.onTranscriptFinal((data: TranscriptFinalEvent) => {
      currentPartial = '';
      transcripts = [...transcripts, data.text];
    });

    unsubStatus = window.api.onStatusUpdate((data: StatusEvent) => {
      isDictating = data.isDictating;
      isModelLoaded = data.isModelLoaded;
      workerHealthy = data.workerHealthy;
    });

    unsubError = window.api.onError((data: ErrorEvent) => {
      errors = [...errors, `[${data.code}] ${data.message}`];
    });

    unsubDisconnected = window.api.onDisconnected(() => {
      connectionStatus = 'Disconnected from Rust';
    });

    // Try to get initial status
    try {
      const status = await window.api.getStatus();
      isDictating = status.isDictating;
      isModelLoaded = status.isModelLoaded;
      workerHealthy = status.workerHealthy;
      connectionStatus = 'Connected';
    } catch (e) {
      connectionStatus = 'Error: ' + (e as Error).message;
    }
  });

  onDestroy(() => {
    unsubTranscriptPartial?.();
    unsubTranscriptFinal?.();
    unsubStatus?.();
    unsubError?.();
    unsubDisconnected?.();
  });

  async function startDictation() {
    try {
      await window.api.startDictation();
      isDictating = true;
    } catch (e) {
      errors = [...errors, 'Failed to start dictation: ' + (e as Error).message];
    }
  }

  async function stopDictation() {
    try {
      await window.api.stopDictation();
      isDictating = false;
      currentPartial = '';
    } catch (e) {
      errors = [...errors, 'Failed to stop dictation: ' + (e as Error).message];
    }
  }

  async function checkHealth() {
    try {
      const result = await window.api.healthCheck();
      connectionStatus = `Healthy: ${new Date(result.timestamp).toLocaleTimeString()}`;
    } catch (e) {
      connectionStatus = 'Error: ' + (e as Error).message;
    }
  }
</script>

<main class="p-8 max-w-4xl mx-auto">
  <h1 class="text-3xl font-bold mb-4">OpenWhisper</h1>
  <p class="text-gray-600 mb-4">Offline dictation for Windows - Phase 2 Protocol Test</p>

  <!-- Status Panel -->
  <div class="bg-gray-100 p-4 rounded mb-4 space-y-2">
    <div class="flex items-center justify-between">
      <span class="font-medium">Connection:</span>
      <span class="text-sm" class:text-green-600={connectionStatus === 'Connected'} class:text-red-600={connectionStatus.startsWith('Error') || connectionStatus.startsWith('Disconnected')}>
        {connectionStatus}
      </span>
    </div>
    <div class="flex items-center justify-between">
      <span class="font-medium">Dictating:</span>
      <span class="text-sm" class:text-green-600={isDictating} class:text-gray-500={!isDictating}>
        {isDictating ? 'Yes' : 'No'}
      </span>
    </div>
    <div class="flex items-center justify-between">
      <span class="font-medium">Model Loaded:</span>
      <span class="text-sm" class:text-green-600={isModelLoaded} class:text-gray-500={!isModelLoaded}>
        {isModelLoaded ? 'Yes' : 'No'}
      </span>
    </div>
    <div class="flex items-center justify-between">
      <span class="font-medium">Worker Healthy:</span>
      <span class="text-sm" class:text-green-600={workerHealthy} class:text-red-600={!workerHealthy}>
        {workerHealthy ? 'Yes' : 'No'}
      </span>
    </div>
  </div>

  <!-- Controls -->
  <div class="flex gap-3 mb-4">
    <button
      class="px-4 py-2 bg-green-500 text-white rounded hover:bg-green-600 disabled:opacity-50 disabled:cursor-not-allowed"
      on:click={startDictation}
      disabled={isDictating}
    >
      Start Dictation
    </button>
    <button
      class="px-4 py-2 bg-red-500 text-white rounded hover:bg-red-600 disabled:opacity-50 disabled:cursor-not-allowed"
      on:click={stopDictation}
      disabled={!isDictating}
    >
      Stop Dictation
    </button>
    <button
      class="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
      on:click={checkHealth}
    >
      Check Health
    </button>
  </div>

  <!-- Live Transcript -->
  <div class="border rounded p-4 mb-4 min-h-[120px]">
    <h2 class="text-lg font-semibold mb-2">Live Transcript</h2>
    <div class="space-y-2">
      {#each transcripts as t}
        <p class="text-gray-800">{t}</p>
      {/each}
      {#if currentPartial}
        <p class="text-gray-500 italic">{currentPartial}</p>
      {/if}
      {#if transcripts.length === 0 && !currentPartial}
        <p class="text-gray-400 italic">No transcripts yet. Start dictation to see mock transcripts.</p>
      {/if}
    </div>
  </div>

  <!-- Errors -->
  {#if errors.length > 0}
    <div class="border border-red-200 bg-red-50 rounded p-4">
      <h2 class="text-lg font-semibold mb-2 text-red-700">Errors</h2>
      <div class="space-y-1">
        {#each errors as err}
          <p class="text-red-600 text-sm">{err}</p>
        {/each}
      </div>
    </div>
  {/if}
</main>
