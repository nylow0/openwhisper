<script lang="ts">
  // PROTOTYPE variant F, "Ledger Plus": A's single column, with D's pieces folded in.
  // Clicking a transcript opens D's detail block inline; settings get D's section tabs and
  // inline language list, rendered as A's document rows. Near-black, blue/cyan accents.
  import { fade, fly, slide } from 'svelte/transition';
  import Flag from './Flag.svelte';
  import WindowControls from './WindowControls.svelte';
  import {
    DEVICES,
    MODELS,
    clearHistory,
    copyText,
    deleteHistoryItem,
    dismissToast,
    draftFromSettings,
    filterLanguages,
    formatTime,
    groupByDay,
    history,
    languageOptions,
    patchSettings,
    restartEngine,
    restarting,
    saveDraft,
    setStartupToggle,
    settings,
    stats,
    toasts,
    toggleDraftLanguage,
    wordCount,
    type LanguageDraft,
    type StartupToggle,
  } from './mock';

  type View = 'history' | 'settings';
  type Section = 'engine' | 'languages' | 'general' | 'about';

  const LAUNCH: StartupToggle = 'launchAtLogin';
  const SHOW: StartupToggle = 'showWindowOnLaunch';
  const GRADIENT = 'linear-gradient(90deg, #3b82f6, #22d3ee)';
  const SECTIONS: Array<{ id: Section; name: string }> = [
    { id: 'engine', name: 'Engine' },
    { id: 'languages', name: 'Languages' },
    { id: 'general', name: 'General' },
    { id: 'about', name: 'About' },
  ];

  let view: View = 'history';
  let section: Section = 'engine';
  let query = '';
  // Latest transcript starts open so the detail block is discoverable.
  let openId: string | null = $history[0]?.id ?? null;
  let copiedId: string | null = null;
  let draft: LanguageDraft = draftFromSettings($settings);
  let pinnedLanguages: LanguageDraft['languages'] = [];
  let languageQuery = '';

  $: needle = query.trim().toLowerCase();
  $: groups = groupByDay(needle ? $history.filter((item) => item.text.toLowerCase().includes(needle)) : $history);
  $: latestId = $history[0]?.id;
  $: spoken = languageOptions($settings.spokenLanguages);
  $: modelName = MODELS.find((model) => model.id === $settings.model)?.name ?? '';
  $: deviceName = $settings.device === 'auto' ? 'the fastest device' : $settings.device.toUpperCase();
  $: dirty =
    draft.auto !== $settings.autoDetectLanguage || draft.languages.join() !== $settings.spokenLanguages.join();

  async function copy(id: string, text: string) {
    if (!(await copyText(text))) return;
    copiedId = id;
    setTimeout(() => (copiedId = copiedId === id ? null : copiedId), 1400);
  }

  function openSection(id: Section) {
    section = id;
    if (id !== 'languages') return;
    draft = draftFromSettings($settings);
    pinnedLanguages = draft.auto ? [] : draft.languages;
    languageQuery = '';
  }
</script>

<div class="relative flex h-full flex-col bg-[#0b0d10] text-[#e8edf2]" style="font-family: 'Plus Jakarta Sans', sans-serif;">
  <header class="drag flex h-11 shrink-0 items-center justify-between pl-8">
    <span class="text-[12px] font-medium text-[#5d6773]">OpenWhisper</span>
    <WindowControls color="#9aa4b0" />
  </header>

  <div class="shrink-0 px-8 pt-2">
    <nav class="flex items-baseline gap-5">
      {#each ['history', 'settings'] as item}
        <button
          class="text-[32px] font-semibold capitalize tracking-[-0.02em] transition-colors {view === item
            ? 'text-white'
            : 'text-[#2c333d] hover:text-[#5d6773]'}"
          on:click={() => (view = item === 'history' ? 'history' : 'settings')}
        >
          {item}
        </button>
      {/each}
    </nav>
    <p class="mt-1.5 flex items-center gap-2 text-[14px] text-[#9aa4b0]">
      <span class="h-1.5 w-1.5 shrink-0 rounded-full {$restarting ? 'animate-breathe bg-[#5d6773]' : 'bg-[#22d3ee] shadow-[0_0_8px_#22d3ee]'}"></span>
      {#if $restarting}
        <span><b class="font-semibold text-white">Restarting.</b> The speech engine is reloading.</span>
      {:else}
        <span class="truncate">
          <b class="font-semibold text-white">Ready.</b> Hold <span class="text-[#7dd3fc]">Ctrl + Win</span> anywhere.
          {modelName} on {deviceName}, {$settings.autoDetectLanguage ? 'any language' : spoken.map((language) => language.nativeName).join(', ')}.
        </span>
      {/if}
    </p>

    {#if view === 'settings'}
      <!-- D: section tabs -->
      <div class="mt-5 inline-flex rounded-xl bg-[#0f1216] p-1 ring-1 ring-[#1b2027]">
        {#each SECTIONS as item}
          <button
            class="rounded-lg px-4 py-1.5 text-[13px] font-medium transition-colors {section === item.id
              ? 'bg-[#172338] text-[#93c5fd]'
              : 'text-[#5d6773] hover:text-[#c5cdd6]'}"
            on:click={() => openSection(item.id)}
          >
            {item.name}
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <div class="scroll-area relative min-h-0 flex-1 overflow-y-auto px-8 pb-10">
    {#if view === 'history'}
      <div class="sticky top-0 z-10 flex items-center gap-4 bg-[#0b0d10] pb-2 pt-5">
        <input
          bind:value={query}
          placeholder="Search"
          class="flex-1 border-b border-[#1b2027] bg-transparent py-2 text-[14px] text-white placeholder:text-[#4a535e] focus:border-[#3b82f6] focus:outline-none"
        />
        {#if $history.length > 0}
          <button class="text-[13px] text-[#5d6773] hover:text-white" on:click={clearHistory}>Clear all</button>
        {/if}
      </div>

      {#if $history.length === 0}
        <div class="flex items-center gap-6 pt-14" in:fade={{ duration: 150 }}>
          <div class="flex gap-2">
            {#each ['Ctrl', 'Win'] as key}
              <kbd class="rounded-xl border border-[#1f3b5c] border-b-[3px] border-b-[#1e4f7a] bg-[#0f1822] px-4 py-2.5 font-sans text-[16px] font-semibold text-[#7dd3fc] shadow-[0_0_24px_rgba(34,211,238,0.12)]">{key}</kbd>
            {/each}
          </div>
          <div>
            <p class="text-[18px] font-semibold">Nothing here yet.</p>
            <p class="mt-1 max-w-[420px] text-[14px] leading-relaxed text-[#7c8792]">
              Hold both keys in any app, speak, and let go. Your words are typed at your cursor and saved here.
            </p>
          </div>
        </div>
      {:else if groups.length === 0}
        <p class="pt-10 text-[14px] text-[#5d6773]">Nothing matches “{query}”.</p>
      {:else}
        {#each groups as group}
          <p class="pb-1 pt-6 text-[12px] font-medium text-[#4a535e]">{group.label}</p>
          {#each group.items as item (item.id)}
            {@const open = openId === item.id}
            <div class="rounded-xl transition-colors {open ? 'my-2 bg-[#0f1216] ring-1 ring-[#1b2027]' : ''}">
              <button class="flex w-full items-start gap-4 rounded-xl py-2.5 text-left {open ? 'px-4 pt-4' : 'hover:bg-[#0f1216]/60'}" on:click={() => (openId = open ? null : item.id)}>
                <span class="mt-[9px] h-[5px] w-[5px] shrink-0 rounded-full {item.id === latestId ? 'bg-[#22d3ee] shadow-[0_0_6px_#22d3ee]' : open ? 'bg-[#3b82f6]' : 'bg-[#2c333d]'}"></span>
                <span class="min-w-0 flex-1 text-[15px] leading-[1.6] {open || item.id === latestId ? 'text-white' : 'text-[#aab3bd]'} {open ? 'whitespace-pre-wrap' : 'line-clamp-2'}">{item.text}</span>
                <span class="w-20 shrink-0 pt-[3px] text-right text-[13px] tabular-nums text-[#4a535e]">{formatTime(item.createdAt)}</span>
              </button>
              {#if open}
                <!-- D: detail block, inline -->
                <div class="px-4 pb-4 pl-[37px]" transition:slide={{ duration: 180 }}>
                  <div class="grid grid-cols-4 gap-6 border-t border-[#1b2027] pt-3">
                    {#each [{ label: 'Language', value: item.language === 'uk' ? 'Ukrainian' : 'English' }, { label: 'Latency', value: `${item.latencyMs} ms` }, { label: 'Words', value: String(wordCount(item.text)) }, { label: 'Characters', value: String(item.text.length) }] as cell}
                      <div>
                        <p class="text-[11px] text-[#5d6773]">{cell.label}</p>
                        <p class="mt-0.5 text-[14px] tabular-nums">{cell.value}</p>
                      </div>
                    {/each}
                  </div>
                  <div class="mt-4 flex gap-2">
                    <button
                      class="rounded-lg px-3.5 py-1.5 text-[13px] font-semibold transition-colors {copiedId === item.id ? 'bg-[#0e3a44] text-[#67e8f9]' : 'bg-[#3b82f6] text-white hover:bg-[#4b8ef8]'}"
                      on:click={() => copy(item.id, item.text)}
                    >
                      {copiedId === item.id ? 'Copied' : 'Copy'}
                    </button>
                    <button class="rounded-lg px-3 py-1.5 text-[13px] text-[#7c8792] hover:bg-[#151a20] hover:text-[#fb7185]" on:click={() => deleteHistoryItem(item.id)}>
                      Delete
                    </button>
                  </div>
                </div>
              {/if}
            </div>
          {/each}
        {/each}
      {/if}
    {:else}
      <div class="pt-6" in:fade={{ duration: 120 }}>
        {#if section === 'engine'}
          <h2 class="border-b border-[#1b2027] pb-2 text-[15px] font-bold">Speech model</h2>
          {#each MODELS as model}
            {@const on = $settings.model === model.id}
            <button disabled={$restarting} class="flex w-full items-center gap-3.5 py-3 text-left disabled:opacity-50" on:click={() => !on && patchSettings({ model: model.id })}>
              <span class="flex h-4 w-4 shrink-0 items-center justify-center rounded-full border {on ? 'border-[#3b82f6]' : 'border-[#2c333d]'}">
                {#if on}<span class="h-2 w-2 rounded-full" style="background: {GRADIENT}"></span>{/if}
              </span>
              <span class="text-[14px] {on ? 'text-white' : 'text-[#9aa4b0]'}">{model.name}</span>
              <span class="text-[13px] text-[#5d6773]">{model.detail}</span>
              <span class="ml-auto text-[12px] tabular-nums text-[#4a535e]">{model.size}</span>
            </button>
          {/each}
          <div class="mt-2 flex items-center justify-between gap-6 py-3">
            <div>
              <p class="text-[14px]">Compute device</p>
              <p class="text-[13px] text-[#5d6773]">Auto uses the GPU when one is available.</p>
            </div>
            <div class="flex gap-4">
              {#each DEVICES as device}
                <button
                  disabled={$restarting}
                  class="text-[14px] underline-offset-[6px] transition-colors {$settings.device === device.id ? 'text-[#7dd3fc] underline decoration-[#22d3ee] decoration-[1.5px]' : 'text-[#4a535e] hover:text-[#9aa4b0]'}"
                  on:click={() => $settings.device !== device.id && patchSettings({ device: device.id })}
                >
                  {device.name}
                </button>
              {/each}
            </div>
          </div>
          <div class="flex items-center justify-between gap-6 py-3">
            <div>
              <p class="text-[14px]">Speech engine</p>
              <p class="text-[13px] text-[#5d6773]">Runs locally. Restart it if dictation stops responding.</p>
            </div>
            <button
              disabled={$restarting}
              class="rounded-lg px-3.5 py-1.5 text-[13px] font-medium text-[#93c5fd] ring-1 ring-[#1f3b5c] hover:bg-[#0f1822] disabled:animate-breathe"
              on:click={restartEngine}
            >
              {$restarting ? 'Restarting…' : 'Restart'}
            </button>
          </div>
        {:else if section === 'languages'}
          {#if $settings.model === 'medium_en_q8'}
            <p class="text-[14px] text-[#7c8792]">The English-only model only understands English. Switch to Multilingual to pick languages.</p>
          {:else}
            <div class="sticky top-0 z-10 flex items-center gap-5 bg-[#0b0d10] pb-2">
              <input
                bind:value={languageQuery}
                placeholder="Search 99 languages"
                class="flex-1 border-b border-[#1b2027] bg-transparent py-2 text-[14px] text-white placeholder:text-[#4a535e] focus:border-[#3b82f6] focus:outline-none"
              />
              <button class="flex items-center gap-2.5 text-[14px]" on:click={() => (draft = { ...draft, auto: !draft.auto })}>
                <span class="relative h-5 w-9 rounded-full transition-colors" style="background: {draft.auto ? GRADIENT : '#1b2027'}">
                  <span class="absolute top-[3px] h-[14px] w-[14px] rounded-full bg-white transition-all {draft.auto ? 'left-[19px]' : 'left-[3px]'}"></span>
                </span>
                Auto-detect
              </button>
            </div>
            <div class="grid grid-cols-2 gap-x-8 pt-1 {draft.auto ? 'opacity-40' : ''} {dirty ? 'pb-14' : ''}">
              {#each filterLanguages(languageQuery, pinnedLanguages) as language (language.id)}
                {@const on = !draft.auto && draft.languages.includes(language.id)}
                <button class="flex items-center gap-3 border-b border-[#12161b] py-2.5 text-left" on:click={() => (draft = toggleDraftLanguage(draft, language.id))}>
                  <Flag {language} size={12} />
                  <span class="text-[14px] {on ? 'text-white' : 'text-[#7c8792]'}">{language.name}</span>
                  {#if language.nativeName !== language.name}<span class="truncate text-[13px] text-[#4a535e]">{language.nativeName}</span>{/if}
                  <span class="ml-auto text-[13px] {on ? 'text-[#22d3ee]' : 'text-transparent'}">✓</span>
                </button>
              {/each}
            </div>
          {/if}
        {:else if section === 'general'}
          <h2 class="border-b border-[#1b2027] pb-2 text-[15px] font-bold">Startup</h2>
          {#each [{ key: LAUNCH, label: 'Launch at login', hint: 'Start OpenWhisper when you sign in to Windows.' }, { key: SHOW, label: 'Open this window on launch', hint: 'Off keeps OpenWhisper in the tray.' }] as row}
            {@const on = $settings[row.key]}
            <button class="flex w-full items-center justify-between gap-6 py-3 text-left" on:click={() => setStartupToggle(row.key, !on)}>
              <span>
                <span class="block text-[14px]">{row.label}</span>
                <span class="block text-[13px] text-[#5d6773]">{row.hint}</span>
              </span>
              <span class="relative h-5 w-9 shrink-0 rounded-full transition-colors" style="background: {on ? GRADIENT : '#1b2027'}">
                <span class="absolute top-[3px] h-[14px] w-[14px] rounded-full bg-white transition-all {on ? 'left-[19px]' : 'left-[3px]'}"></span>
              </span>
            </button>
          {/each}
          <h2 class="mt-7 border-b border-[#1b2027] pb-2 text-[15px] font-bold">Dictation</h2>
          <div class="flex items-center justify-between gap-6 py-3">
            <div>
              <p class="text-[14px]">Hold to dictate</p>
              <p class="text-[13px] text-[#5d6773]">Works in any app. Release to type the text.</p>
            </div>
            <span class="flex gap-1">
              {#each ['Ctrl', 'Win'] as key}
                <kbd class="rounded-md border border-[#1f3b5c] border-b-2 border-b-[#1e4f7a] bg-[#0f1822] px-2 py-0.5 font-sans text-[12px] text-[#7dd3fc]">{key}</kbd>
              {/each}
            </span>
          </div>
        {:else}
          <div class="grid grid-cols-3 gap-6 rounded-xl bg-[#0f1216] p-5 ring-1 ring-[#1b2027]">
            {#each [{ label: 'Transcripts', value: String($stats.count) }, { label: 'Words dictated', value: String($stats.words) }, { label: 'Average latency', value: $stats.avgLatencyMs ? `${$stats.avgLatencyMs} ms` : '—' }] as cell}
              <div>
                <p class="text-[12px] text-[#5d6773]">{cell.label}</p>
                <p class="mt-1 bg-clip-text text-[24px] font-semibold tabular-nums text-transparent" style="background-image: {GRADIENT}">{cell.value}</p>
              </div>
            {/each}
          </div>
          <p class="mt-6 text-[13px] text-[#7c8792]">OpenWhisper 0.1.0</p>
          <p class="text-[13px] text-[#4a535e]">Fully offline. Audio never leaves this computer.</p>
        {/if}
      </div>
    {/if}
  </div>

  {#if view === 'settings' && section === 'languages' && dirty}
    <footer class="flex shrink-0 items-center gap-5 border-t border-[#1b2027] px-8 py-3.5" transition:fly={{ y: 20, duration: 180 }}>
      <p class="flex-1 text-[13px] text-[#9aa4b0]">
        <b class="font-semibold text-white">{draft.auto ? 'Auto-detect.' : `${draft.languages.length} selected.`}</b> Applying restarts the engine.
      </p>
      <button class="text-[13px] text-[#7c8792] hover:text-white" on:click={() => openSection('languages')}>Discard</button>
      <button class="rounded-lg bg-[#3b82f6] px-4 py-1.5 text-[13px] font-semibold text-white hover:bg-[#4b8ef8]" on:click={() => saveDraft(draft)}>Apply</button>
    </footer>
  {/if}

  <!-- Toasts: A's full-width sentence with D's action -->
  <div class="pointer-events-none absolute left-8 right-8 z-50 flex flex-col gap-2 {view === 'settings' && section === 'languages' && dirty ? 'bottom-[72px]' : 'bottom-5'}">
    {#each $toasts as toast (toast.id)}
      <div
        in:fly={{ y: 12, duration: 200 }}
        out:fade={{ duration: 150 }}
        class="pointer-events-auto flex items-center gap-4 rounded-2xl border border-[#1b2027] bg-[#11151a] px-5 py-3.5 shadow-[0_16px_50px_rgba(0,0,0,0.6)]"
      >
        <p class="flex-1 text-[14px] leading-relaxed text-[#aab3bd]"><b class="font-bold text-white">Needs attention.</b> {toast.message}</p>
        <button
          class="shrink-0 rounded-lg bg-[#3b82f6] px-3 py-1.5 text-[12px] font-semibold text-white"
          on:click={() => {
            restartEngine();
            dismissToast(toast.id);
          }}
        >
          Restart
        </button>
        <button class="shrink-0 text-[13px] text-[#5d6773] hover:text-white" on:click={() => dismissToast(toast.id)}>Dismiss</button>
      </div>
    {/each}
  </div>
</div>
