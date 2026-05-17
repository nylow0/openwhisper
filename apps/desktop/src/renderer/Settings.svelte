<script lang="ts">
  import { getContext, onMount } from 'svelte';
  import type { AppSettings, AsrDevice, AsrModel } from '../shared/types';

  const api = window.api;
  const pushToast = getContext<(message: string) => void>('pushToast');

  const MODELS: Array<{ id: AsrModel; name: string; detail: string }> = [
    { id: 'base_en_q8', name: 'Base', detail: 'Fastest · ~4% word error rate' },
    { id: 'medium_en_q8', name: 'Medium', detail: 'Most accurate · ~2% word error rate' },
  ];
  const DEVICES: Array<{ id: AsrDevice; name: string }> = [
    { id: 'auto', name: 'Auto' },
    { id: 'cpu', name: 'CPU' },
    { id: 'gpu', name: 'GPU' },
  ];

  let settings: AppSettings | null = null;
  let restarting = false;

  onMount(async () => {
    if (!api) return;
    settings = await api.getSettings();
  });

  async function patch(partial: Partial<AppSettings>) {
    if (!api || !settings) return;
    settings = { ...settings, ...partial };
    settings = await api.saveSettings(partial);
  }

  async function applyEngineChange(label: string) {
    if (!api) return;
    restarting = true;
    const result = await api.restartEngine();
    restarting = false;
    pushToast(
      result.ok
        ? `${label} — speech engine restarted.`
        : `Engine restart failed: ${result.error ?? 'unknown error'}`
    );
  }

  async function chooseModel(model: AsrModel) {
    if (!settings || restarting || settings.model === model) return;
    await patch({ model });
    await applyEngineChange('Model updated');
  }

  async function chooseDevice(device: AsrDevice) {
    if (!settings || restarting || settings.device === device) return;
    await patch({ device });
    await applyEngineChange('Compute device updated');
  }
</script>

<div class="scroll-area h-full overflow-y-auto px-7 py-6">
  <h1 class="text-lg font-semibold text-zinc-100">Settings</h1>
  <p class="mt-0.5 text-[12px] text-zinc-500">Configure the speech engine and how OpenWhisper starts.</p>

  <div class="mt-6 space-y-6">
    <!-- Speech model -->
    <section>
      <h2 class="mb-2 text-[11px] font-semibold uppercase tracking-wider text-zinc-500">Speech model</h2>
      <div class="rounded-xl border border-zinc-800 bg-zinc-900 p-4">
        <div class="flex gap-2.5">
          {#each MODELS as model}
            <button
              type="button"
              on:click={() => chooseModel(model.id)}
              disabled={restarting}
              class="flex-1 rounded-lg border p-3 text-left transition-colors disabled:opacity-60 {settings?.model ===
              model.id
                ? 'border-indigo-500 bg-indigo-500/10'
                : 'border-zinc-800 bg-zinc-950 hover:border-zinc-700'}"
            >
              <div class="flex items-center justify-between">
                <span class="text-[13px] font-semibold text-zinc-100">{model.name}</span>
                {#if settings?.model === model.id}
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" class="h-3.5 w-3.5 text-indigo-400" aria-hidden="true">
                    <path d="M20 6 9 17l-5-5" />
                  </svg>
                {/if}
              </div>
              <p class="mt-0.5 text-[11px] text-zinc-500">{model.detail}</p>
            </button>
          {/each}
        </div>

        <div class="mt-4 flex items-center justify-between gap-3">
          <div>
            <p class="text-[13px] font-medium text-zinc-200">Compute device</p>
            <p class="text-[11px] text-zinc-500">Auto prefers the GPU when it's available.</p>
          </div>
          <div class="inline-flex shrink-0 rounded-lg border border-zinc-800 bg-zinc-950 p-0.5">
            {#each DEVICES as device}
              <button
                type="button"
                on:click={() => chooseDevice(device.id)}
                disabled={restarting}
                class="rounded-md px-3 py-1.5 text-[12px] font-medium transition-colors disabled:opacity-60 {settings?.device ===
                device.id
                  ? 'bg-zinc-800 text-zinc-100'
                  : 'text-zinc-500 hover:text-zinc-300'}"
              >
                {device.name}
              </button>
            {/each}
          </div>
        </div>

        <p class="mt-3 flex items-center gap-1.5 text-[11px] text-zinc-500">
          {#if restarting}
            <svg viewBox="0 0 24 24" fill="none" class="h-3.5 w-3.5 animate-spin text-indigo-400" aria-hidden="true">
              <circle cx="12" cy="12" r="9" stroke="currentColor" stroke-width="3" class="opacity-20" />
              <path d="M21 12a9 9 0 0 0-9-9" stroke="currentColor" stroke-width="3" stroke-linecap="round" />
            </svg>
            Restarting the speech engine…
          {:else}
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-3.5 w-3.5" aria-hidden="true">
              <circle cx="12" cy="12" r="9" />
              <path d="M12 11v5M12 8h.01" />
            </svg>
            Changing the model or device restarts the speech engine.
          {/if}
        </p>
      </div>
    </section>

    <!-- Startup -->
    <section>
      <h2 class="mb-2 text-[11px] font-semibold uppercase tracking-wider text-zinc-500">Startup</h2>
      <div class="divide-y divide-zinc-800 rounded-xl border border-zinc-800 bg-zinc-900">
        <div class="flex items-center justify-between gap-4 p-4">
          <div>
            <p class="text-[13px] font-medium text-zinc-200">Launch at login</p>
            <p class="text-[11px] text-zinc-500">Start OpenWhisper automatically when you sign in.</p>
          </div>
          <button
            type="button"
            role="switch"
            aria-label="Launch at login"
            aria-checked={settings?.launchAtLogin ?? false}
            on:click={() => patch({ launchAtLogin: !(settings?.launchAtLogin ?? false) })}
            class="relative h-[22px] w-10 shrink-0 rounded-full transition-colors {settings?.launchAtLogin
              ? 'bg-indigo-500'
              : 'bg-zinc-700'}"
          >
            <span
              class="absolute top-[3px] h-4 w-4 rounded-full bg-white transition-all {settings?.launchAtLogin
                ? 'left-[21px]'
                : 'left-[3px]'}"
            ></span>
          </button>
        </div>
        <div class="flex items-center justify-between gap-4 p-4">
          <div>
            <p class="text-[13px] font-medium text-zinc-200">Open window on launch</p>
            <p class="text-[11px] text-zinc-500">Off keeps OpenWhisper in the tray only when it starts.</p>
          </div>
          <button
            type="button"
            role="switch"
            aria-label="Open window on launch"
            aria-checked={settings?.showWindowOnLaunch ?? false}
            on:click={() => patch({ showWindowOnLaunch: !(settings?.showWindowOnLaunch ?? false) })}
            class="relative h-[22px] w-10 shrink-0 rounded-full transition-colors {settings?.showWindowOnLaunch
              ? 'bg-indigo-500'
              : 'bg-zinc-700'}"
          >
            <span
              class="absolute top-[3px] h-4 w-4 rounded-full bg-white transition-all {settings?.showWindowOnLaunch
                ? 'left-[21px]'
                : 'left-[3px]'}"
            ></span>
          </button>
        </div>
      </div>
    </section>

    <!-- Speech engine -->
    <section>
      <h2 class="mb-2 text-[11px] font-semibold uppercase tracking-wider text-zinc-500">Speech engine</h2>
      <div class="flex items-center justify-between gap-4 rounded-xl border border-zinc-800 bg-zinc-900 p-4">
        <div>
          <p class="text-[13px] font-medium text-zinc-200">Background engine</p>
          <p class="text-[11px] text-zinc-500">
            Transcription runs in a local helper process. Restart it if it stops responding.
          </p>
        </div>
        <button
          type="button"
          on:click={() => applyEngineChange('Restart requested')}
          disabled={restarting}
          class="inline-flex shrink-0 items-center gap-1.5 rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2 text-[12px] font-medium text-zinc-200 transition-colors hover:bg-zinc-700 disabled:opacity-60"
        >
          {#if restarting}
            <svg viewBox="0 0 24 24" fill="none" class="h-3.5 w-3.5 animate-spin" aria-hidden="true">
              <circle cx="12" cy="12" r="9" stroke="currentColor" stroke-width="3" class="opacity-20" />
              <path d="M21 12a9 9 0 0 0-9-9" stroke="currentColor" stroke-width="3" stroke-linecap="round" />
            </svg>
            Restarting…
          {:else}
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-3.5 w-3.5" aria-hidden="true">
              <path d="M21 12a9 9 0 1 1-3-6.7" />
              <path d="M21 3v5h-5" />
            </svg>
            Restart engine
          {/if}
        </button>
      </div>
    </section>

    <!-- Dictation shortcut -->
    <section>
      <h2 class="mb-2 text-[11px] font-semibold uppercase tracking-wider text-zinc-500">Dictation</h2>
      <div class="flex items-center justify-between gap-4 rounded-xl border border-zinc-800 bg-zinc-900 p-4">
        <div>
          <p class="text-[13px] font-medium text-zinc-200">Shortcut</p>
          <p class="text-[11px] text-zinc-500">Hold anywhere to record, release to insert the text.</p>
        </div>
        <div class="flex shrink-0 items-center gap-1">
          <kbd class="rounded-md border border-zinc-700 bg-zinc-800 px-2 py-1 text-[11px] font-semibold text-zinc-300">Ctrl</kbd>
          <span class="text-[11px] text-zinc-600">+</span>
          <kbd class="rounded-md border border-zinc-700 bg-zinc-800 px-2 py-1 text-[11px] font-semibold text-zinc-300">Win</kbd>
        </div>
      </div>
    </section>

    <!-- About -->
    <section>
      <h2 class="mb-2 text-[11px] font-semibold uppercase tracking-wider text-zinc-500">About</h2>
      <div class="rounded-xl border border-zinc-800 bg-zinc-900 p-4">
        <p class="text-[13px] font-medium text-zinc-200">OpenWhisper 0.1.0</p>
        <p class="mt-0.5 text-[11px] text-zinc-500">
          Fully offline speech-to-text. Your audio never leaves this computer.
        </p>
      </div>
    </section>
  </div>
</div>
