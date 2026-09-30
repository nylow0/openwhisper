<script lang="ts">
  // Main window shell. The big "History / Settings" title is the navigation; the
  // sentence under it reports the engine's live status. Views render below it.
  import { onDestroy, onMount, setContext } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import History from './History.svelte';
  import Settings from './Settings.svelte';
  import { DEVICES, MODELS, engineStatus, initAppState, restartEngine, settings } from './app-state';
  import { languageOptions } from './languages';
  import type { ErrorEvent } from '../shared/types';

  type View = 'history' | 'settings';
  type ToastAction = { label: string; run: () => void };
  type Toast = { id: number; message: string; code?: string; action?: ToastAction };

  const api = window.api;
  const VIEWS: View[] = ['history', 'settings'];

  let view: View = 'history';
  let toasts: Toast[] = [];
  let nextToastId = 1;

  function pushToast(message: string, options: { code?: string; action?: ToastAction } = {}) {
    const id = nextToastId;
    nextToastId += 1;
    toasts = [{ id, message, ...options }, ...toasts].slice(0, 3);
    setTimeout(() => dismissToast(id), 8000);
  }

  function dismissToast(id: number) {
    toasts = toasts.filter((toast) => toast.id !== id);
  }

  async function restartFromUi() {
    const result = await restartEngine();
    if (!result.ok) pushToast(`Engine restart failed: ${result.error ?? 'unknown error'}`);
  }

  const RESTART_ACTION: ToastAction = { label: 'Restart engine', run: restartFromUi };

  // Settings raises toasts through this.
  setContext('pushToast', pushToast);

  let cleanup: (() => void) | undefined;
  let unsubError: (() => void) | undefined;

  onMount(() => {
    if (!api) {
      pushToast('Preload API unavailable. Rebuild and restart the app.');
      return;
    }
    cleanup = initAppState(() => pushToast('Lost connection to the speech engine.', { action: RESTART_ACTION }));
    unsubError = api.onError((data: ErrorEvent) => pushToast(data.message, { code: data.code }));
  });

  onDestroy(() => {
    cleanup?.();
    unsubError?.();
  });

  $: modelName = MODELS.find((model) => model.id === $settings?.model)?.name ?? '';
  $: deviceName =
    $settings?.device === 'auto' ? 'GPU when available' : DEVICES.find((device) => device.id === $settings?.device)?.name;
  $: languagesText = !$settings
    ? ''
    : $settings.model === 'medium_en_q8'
      ? 'English'
      : $settings.autoDetectLanguage
        ? 'any language'
        : languageOptions($settings.spokenLanguages)
            .map((language) => language.nativeName)
            .join(', ');
</script>

<div class="relative flex h-screen flex-col overflow-hidden bg-ink-950 text-ink-50">
  <!-- Title bar: draggable, native window controls are overlaid on the right. -->
  <header class="drag flex h-11 shrink-0 select-none items-center pl-8">
    <span class="text-[12px] font-medium text-ink-400">OpenWhisper</span>
  </header>

  <div class="shrink-0 px-8 pt-1">
    <nav class="flex items-baseline gap-5">
      {#each VIEWS as item}
        <button
          type="button"
          class="text-[32px] font-semibold capitalize leading-tight tracking-[-0.02em] transition-colors {view === item
            ? 'text-white'
            : 'text-ink-600 hover:text-ink-400'}"
          aria-current={view === item ? 'page' : undefined}
          on:click={() => (view = item)}
        >
          {item}
        </button>
      {/each}
    </nav>

    <p class="mt-1.5 flex items-center gap-2.5 text-[14px] text-ink-200">
      <span
        class="h-1.5 w-1.5 shrink-0 rounded-full {$engineStatus === 'ready'
          ? 'bg-cyan-400 shadow-[0_0_8px_#22d3ee]'
          : $engineStatus === 'offline'
            ? 'bg-rose-400'
            : 'animate-breathe bg-ink-400'}"
      ></span>
      <span class="min-w-0 truncate">
        {#if $engineStatus === 'starting'}
          <b class="font-semibold text-white">Starting.</b> Connecting to the speech engine…
        {:else if $engineStatus === 'restarting'}
          <b class="font-semibold text-white">Restarting.</b> The speech engine is reloading.
        {:else if $engineStatus === 'offline'}
          <b class="font-semibold text-white">Offline.</b> The speech engine isn't running.
        {:else}
          <b class="font-semibold text-white">Ready.</b> Hold <span class="text-sky-300">Ctrl + Win</span> anywhere to
          dictate.
          {#if $settings}<span class="text-ink-400">{modelName} · {deviceName} · {languagesText}</span>{/if}
        {/if}
      </span>
      {#if $engineStatus === 'offline'}
        <button
          type="button"
          class="shrink-0 text-sky-300 underline decoration-sky-900 underline-offset-4 hover:decoration-sky-300"
          on:click={restartFromUi}
        >
          Restart
        </button>
      {/if}
    </p>
  </div>

  <div class="flex min-h-0 flex-1 flex-col">
    {#if view === 'history'}
      <History />
    {:else}
      <Settings />
    {/if}
  </div>

  <!-- Toasts -->
  <div class="pointer-events-none absolute bottom-5 left-8 right-8 z-50 flex flex-col gap-2">
    {#each toasts as toast (toast.id)}
      <div
        in:fly={{ y: 12, duration: 200 }}
        out:fade={{ duration: 150 }}
        class="pointer-events-auto flex items-center gap-4 rounded-2xl border border-ink-700 bg-ink-850 px-5 py-3.5 shadow-[0_16px_50px_rgba(0,0,0,0.6)]"
        role="alert"
      >
        <p class="min-w-0 flex-1 text-[14px] leading-relaxed text-ink-150">
          <b class="font-bold text-white">Needs attention.</b>
          {toast.message}
          {#if toast.code}<span class="ml-1 text-[12px] text-ink-500">{toast.code}</span>{/if}
        </p>
        {#if toast.action}
          {@const action = toast.action}
          <button
            type="button"
            class="shrink-0 rounded-lg bg-blue-500 px-3 py-1.5 text-[12px] font-semibold text-white hover:bg-blue-400"
            on:click={() => {
              action.run();
              dismissToast(toast.id);
            }}
          >
            {action.label}
          </button>
        {/if}
        <button type="button" class="shrink-0 text-[13px] text-ink-400 hover:text-white" on:click={() => dismissToast(toast.id)}>
          Dismiss
        </button>
      </div>
    {/each}
  </div>
</div>
