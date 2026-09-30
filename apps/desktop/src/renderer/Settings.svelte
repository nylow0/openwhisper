<script lang="ts">
  // Settings, split into section tabs. Each section reads like a document: a bold
  // heading, then label/hint rows with their control on the right. Model, device,
  // and language changes restart the speech engine.
  import { getContext } from 'svelte';
  import { fade } from 'svelte/transition';
  import LanguageSettings from './LanguageSettings.svelte';
  import Toggle from './Toggle.svelte';
  import { DEVICES, MODELS, engineStatus, history, saveSettings, settings } from './app-state';
  import type { AsrDevice, AsrModel } from '../shared/types';

  type Section = 'engine' | 'languages' | 'general' | 'about';

  const SECTIONS: Array<{ id: Section; name: string }> = [
    { id: 'engine', name: 'Engine' },
    { id: 'languages', name: 'Languages' },
    { id: 'general', name: 'General' },
    { id: 'about', name: 'About' },
  ];

  const restartEngine = getContext<() => Promise<void>>('restartEngine');

  let section: Section = 'engine';

  $: restarting = $engineStatus === 'restarting';
  $: latencies = $history.flatMap((item) => (item.latencyMs === null ? [] : [item.latencyMs]));
  $: stats = [
    { label: 'Transcripts', value: String($history.length) },
    {
      label: 'Words dictated',
      value: String($history.reduce((sum, item) => sum + item.text.split(/\s+/).filter(Boolean).length, 0)),
    },
    {
      label: 'Average latency',
      value: latencies.length ? `${Math.round(latencies.reduce((a, b) => a + b, 0) / latencies.length)} ms` : '—',
    },
  ];

  async function chooseModel(model: AsrModel) {
    if (!$settings || restarting || $settings.model === model) return;
    await saveSettings({ model });
    await restartEngine();
    // The multilingual model is only as good as its language list; show it next.
    if (model === 'large_v3_turbo_q8') section = 'languages';
  }

  async function chooseDevice(device: AsrDevice) {
    if (!$settings || restarting || $settings.device === device) return;
    await saveSettings({ device });
    await restartEngine();
  }
</script>

<div class="shrink-0 px-8 pt-5">
  <div class="inline-flex rounded-xl bg-ink-900 p-1 ring-1 ring-ink-700" role="tablist">
    {#each SECTIONS as item}
      <button
        type="button"
        role="tab"
        aria-selected={section === item.id}
        class="rounded-lg px-4 py-1.5 text-[13px] font-medium transition-colors {section === item.id
          ? 'bg-[#172338] text-blue-300'
          : 'text-ink-400 hover:text-ink-100'}"
        on:click={() => (section = item.id)}
      >
        {item.name}
      </button>
    {/each}
  </div>
</div>

{#if section === 'languages'}
  <LanguageSettings on:switchModel={() => chooseModel('large_v3_turbo_q8')} />
{:else}
  <div class="scroll-area min-h-0 flex-1 overflow-y-auto px-8 pb-10 pt-6">
    {#key section}
      <div in:fade={{ duration: 120 }}>
        {#if section === 'engine'}
          <h2 class="border-b border-ink-700 pb-2 text-[15px] font-bold">Speech model</h2>
          {#each MODELS as model}
            {@const on = $settings?.model === model.id}
            <button
              type="button"
              role="radio"
              aria-checked={on}
              disabled={restarting}
              class="flex w-full items-center gap-3.5 py-3 text-left disabled:opacity-50"
              on:click={() => chooseModel(model.id)}
            >
              <span class="flex h-4 w-4 shrink-0 items-center justify-center rounded-full border {on ? 'border-blue-500' : 'border-ink-600'}">
                {#if on}<span class="h-2 w-2 rounded-full bg-accent"></span>{/if}
              </span>
              <span class="text-[14px] {on ? 'text-white' : 'text-ink-200'}">{model.name}</span>
              <span class="text-[13px] text-ink-400">{model.detail}</span>
              <span class="ml-auto text-[12px] tabular-nums text-ink-500">{model.size}</span>
            </button>
          {/each}

          <div class="mt-2 flex items-center justify-between gap-6 py-3">
            <div>
              <p class="text-[14px]">Compute device</p>
              <p class="text-[13px] text-ink-400">Auto uses the GPU when one is available.</p>
            </div>
            <div class="flex gap-4">
              {#each DEVICES as device}
                <button
                  type="button"
                  disabled={restarting}
                  aria-pressed={$settings?.device === device.id}
                  class="text-[14px] underline-offset-[6px] transition-colors disabled:opacity-50 {$settings?.device === device.id
                    ? 'text-sky-300 underline decoration-cyan-400 decoration-[1.5px]'
                    : 'text-ink-500 hover:text-ink-200'}"
                  on:click={() => chooseDevice(device.id)}
                >
                  {device.name}
                </button>
              {/each}
            </div>
          </div>

          <div class="flex items-center justify-between gap-6 py-3">
            <div>
              <p class="text-[14px]">Speech engine</p>
              <p class="text-[13px] text-ink-400">Runs locally. Restart it if dictation stops responding.</p>
            </div>
            <button
              type="button"
              disabled={restarting}
              class="rounded-lg px-3.5 py-1.5 text-[13px] font-medium text-blue-300 ring-1 ring-[#1f3b5c] hover:bg-[#0f1822] disabled:animate-breathe"
              on:click={restartEngine}
            >
              {restarting ? 'Restarting…' : 'Restart'}
            </button>
          </div>
        {:else if section === 'general'}
          <h2 class="border-b border-ink-700 pb-2 text-[15px] font-bold">Startup</h2>
          <div class="flex items-center justify-between gap-6 py-3">
            <div>
              <p class="text-[14px]">Launch at login</p>
              <p class="text-[13px] text-ink-400">Start OpenWhisper when you sign in to Windows.</p>
            </div>
            <Toggle
              label="Launch at login"
              checked={$settings?.launchAtLogin ?? false}
              on:change={(event) => saveSettings({ launchAtLogin: event.detail })}
            />
          </div>
          <div class="flex items-center justify-between gap-6 py-3">
            <div>
              <p class="text-[14px]">Open this window on launch</p>
              <p class="text-[13px] text-ink-400">Off keeps OpenWhisper in the tray.</p>
            </div>
            <Toggle
              label="Open this window on launch"
              checked={$settings?.showWindowOnLaunch ?? false}
              on:change={(event) => saveSettings({ showWindowOnLaunch: event.detail })}
            />
          </div>

          <h2 class="mt-7 border-b border-ink-700 pb-2 text-[15px] font-bold">Dictation</h2>
          <div class="flex items-center justify-between gap-6 py-3">
            <div>
              <p class="text-[14px]">Hold to dictate</p>
              <p class="text-[13px] text-ink-400">Works in any app. Release to type the text.</p>
            </div>
            <span class="flex gap-1">
              {#each ['Ctrl', 'Win'] as key}
                <kbd class="rounded-md border border-[#1f3b5c] border-b-2 border-b-[#1e4f7a] bg-[#0f1822] px-2 py-0.5 font-sans text-[12px] text-sky-300">
                  {key}
                </kbd>
              {/each}
            </span>
          </div>
        {:else}
          <div class="grid grid-cols-3 gap-6 rounded-xl bg-ink-900 p-5 ring-1 ring-ink-700">
            {#each stats as cell}
              <div>
                <p class="text-[12px] text-ink-400">{cell.label}</p>
                <p class="mt-1 bg-accent bg-clip-text text-[24px] font-semibold tabular-nums text-transparent">{cell.value}</p>
              </div>
            {/each}
          </div>
          <p class="mt-6 text-[13px] text-ink-300">OpenWhisper 0.1.0</p>
          <p class="text-[13px] text-ink-500">Fully offline speech-to-text. Your audio never leaves this computer.</p>
        {/if}
      </div>
    {/key}
  </div>
{/if}
