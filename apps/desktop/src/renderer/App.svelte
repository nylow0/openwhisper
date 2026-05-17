<script lang="ts">
  import { onDestroy, onMount, setContext } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import History from './History.svelte';
  import Settings from './Settings.svelte';
  import type { ErrorEvent } from '../shared/types';

  type Toast = { id: number; message: string };
  type View = 'history' | 'settings';

  const api = window.api;

  let view: View = 'history';
  let toasts: Toast[] = [];
  let nextToastId = 1;

  function pushToast(message: string) {
    const id = nextToastId;
    nextToastId += 1;
    toasts = [{ id, message }, ...toasts].slice(0, 4);
    setTimeout(() => dismissToast(id), 6000);
  }

  function dismissToast(id: number) {
    toasts = toasts.filter((toast) => toast.id !== id);
  }

  // Children (History / Settings) raise error toasts through this.
  setContext('pushToast', pushToast);

  let unsubError: (() => void) | undefined;
  let unsubDisconnected: (() => void) | undefined;

  onMount(() => {
    if (!api) {
      pushToast('Preload API unavailable. Rebuild and restart the app.');
      return;
    }
    unsubError = api.onError((data: ErrorEvent) => pushToast(`[${data.code}] ${data.message}`));
    unsubDisconnected = api.onDisconnected(() =>
      pushToast('Lost connection to the speech engine.')
    );
  });

  onDestroy(() => {
    unsubError?.();
    unsubDisconnected?.();
  });

  const NAV: Array<{ id: View; label: string }> = [
    { id: 'history', label: 'History' },
    { id: 'settings', label: 'Settings' },
  ];
</script>

<div class="flex h-screen flex-col overflow-hidden bg-zinc-950 text-zinc-100">
  <!-- Title bar — draggable, native window controls overlaid at right -->
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

  <div class="flex min-h-0 flex-1">
    <!-- Sidebar navigation -->
    <nav class="flex w-44 shrink-0 flex-col gap-1 border-r border-zinc-800/80 bg-zinc-950 p-3">
      {#each NAV as item}
        <button
          type="button"
          class="flex items-center gap-2.5 rounded-lg px-3 py-2 text-[13px] font-medium transition-colors {view ===
          item.id
            ? 'bg-zinc-800 text-zinc-100'
            : 'text-zinc-500 hover:bg-zinc-900 hover:text-zinc-300'}"
          on:click={() => (view = item.id)}
        >
          {#if item.id === 'history'}
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
              <circle cx="12" cy="12" r="9" />
              <path d="M12 7v5l3 2" />
            </svg>
          {:else}
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
              <circle cx="12" cy="12" r="3" />
              <path d="M12 2.5a1.7 1.7 0 0 1 1.7 1.4l.2 1.2a6.7 6.7 0 0 1 1.6.9l1.1-.4a1.7 1.7 0 0 1 2 .8l.7 1.2a1.7 1.7 0 0 1-.3 2.1l-.9.8a6.8 6.8 0 0 1 0 1.8l.9.8a1.7 1.7 0 0 1 .3 2.1l-.7 1.2a1.7 1.7 0 0 1-2 .8l-1.1-.4a6.7 6.7 0 0 1-1.6.9l-.2 1.2a1.7 1.7 0 0 1-1.7 1.4h-1.4a1.7 1.7 0 0 1-1.7-1.4l-.2-1.2a6.7 6.7 0 0 1-1.6-.9l-1.1.4a1.7 1.7 0 0 1-2-.8l-.7-1.2a1.7 1.7 0 0 1 .3-2.1l.9-.8a6.8 6.8 0 0 1 0-1.8l-.9-.8a1.7 1.7 0 0 1-.3-2.1l.7-1.2a1.7 1.7 0 0 1 2-.8l1.1.4a6.7 6.7 0 0 1 1.6-.9l.2-1.2A1.7 1.7 0 0 1 10.6 2.5Z" />
            </svg>
          {/if}
          {item.label}
        </button>
      {/each}
    </nav>

    <!-- Active view -->
    <div class="min-w-0 flex-1 overflow-hidden">
      {#if view === 'history'}
        <History />
      {:else}
        <Settings />
      {/if}
    </div>
  </div>

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
        <p class="min-w-0 flex-1 text-[12px] font-medium leading-snug text-zinc-200">{toast.message}</p>
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
