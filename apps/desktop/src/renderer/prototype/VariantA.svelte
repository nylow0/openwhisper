<script lang="ts">
  // PROTOTYPE variant A, "Ledger": pure black, no cards, no sidebar. The big title doubles as
  // navigation, history is a plain dotted timeline, settings read like a document.
  import { fade, fly } from 'svelte/transition';
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
    settings,
    stats,
    toasts,
    toggleDraftLanguage,
    wordCount,
    setStartupToggle,
    type LanguageDraft,
    type StartupToggle,
  } from './mock';

  const LAUNCH: StartupToggle = 'launchAtLogin';
  const SHOW: StartupToggle = 'showWindowOnLaunch';

  type View = 'history' | 'settings' | 'languages';

  let view: View = 'history';
  let query = '';
  let expandedId: string | null = null;
  let copiedId: string | null = null;
  let draft: LanguageDraft = { languages: [], auto: false };
  let languageQuery = '';
  let pinnedLanguages: LanguageDraft['languages'] = [];

  $: needle = query.trim().toLowerCase();
  $: groups = groupByDay(needle ? $history.filter((item) => item.text.toLowerCase().includes(needle)) : $history);
  $: selected = languageOptions($settings.spokenLanguages);
  $: modelName = MODELS.find((model) => model.id === $settings.model)?.name ?? '';
  $: deviceName = $settings.device === 'auto' ? 'the fastest device' : $settings.device.toUpperCase();
  $: filteredLanguages = filterLanguages(languageQuery, pinnedLanguages);

  async function copy(id: string, text: string) {
    if (!(await copyText(text))) return;
    copiedId = id;
    setTimeout(() => (copiedId = copiedId === id ? null : copiedId), 1400);
  }

  function openLanguages() {
    draft = draftFromSettings($settings);
    pinnedLanguages = draft.auto ? [] : draft.languages;
    languageQuery = '';
    view = 'languages';
  }
</script>

<div class="relative flex h-full flex-col bg-black text-[#fafafa]" style="font-family: 'Plus Jakarta Sans', sans-serif;">
  <!-- Title bar -->
  <header class="drag flex h-11 shrink-0 items-center justify-between pl-8">
    <span class="text-[12px] font-medium text-[#5c5c5c]">OpenWhisper</span>
    <WindowControls color="#a3a3a3" />
  </header>

  <!-- Title-as-navigation + status sentence -->
  <div class="shrink-0 px-8 pt-3">
    {#if view === 'languages'}
      <div class="flex items-baseline gap-3">
        <button class="text-[15px] text-[#8a8a8a] hover:text-white" on:click={() => (view = 'settings')}>Settings</button>
        <h1 class="text-[32px] font-semibold tracking-[-0.02em]">Languages</h1>
      </div>
    {:else}
      <nav class="flex items-baseline gap-5">
        {#each ['history', 'settings'] as item}
          <button
            class="text-[32px] font-semibold capitalize tracking-[-0.02em] transition-colors {view === item
              ? 'text-white'
              : 'text-[#333] hover:text-[#6b6b6b]'}"
            on:click={() => (view = item === 'history' ? 'history' : 'settings')}
          >
            {item}
          </button>
        {/each}
      </nav>
    {/if}
    <p class="mt-2 text-[14px] text-[#a3a3a3]">
      {#if $restarting}
        <b class="font-semibold text-white">Restarting.</b> The speech engine is reloading, this takes a few seconds.
      {:else if view === 'languages'}
        {#if draft.auto}
          <b class="font-semibold text-white">Auto-detect.</b> Whisper listens for all 99 languages.
        {:else}
          <b class="font-semibold text-white">{draft.languages.length} selected.</b> Whisper only listens for these.
        {/if}
      {:else}
        <b class="font-semibold text-white">Ready.</b> Hold Ctrl + Win anywhere to dictate. {modelName} on {deviceName},
        {$settings.autoDetectLanguage ? 'any language' : selected.map((language) => language.nativeName).join(', ')}.
      {/if}
    </p>
  </div>

  <div class="scroll-area relative min-h-0 flex-1 overflow-y-auto px-8 pb-10">
    {#if view === 'history'}
      <div class="sticky top-0 z-10 flex items-center gap-4 bg-black pb-2 pt-6">
        <input
          bind:value={query}
          placeholder="Search"
          class="flex-1 border-b border-[#1f1f1f] bg-transparent py-2 text-[14px] text-white placeholder:text-[#4d4d4d] focus:border-[#525252] focus:outline-none"
        />
        {#if $history.length > 0}
          <button class="text-[13px] text-[#6b6b6b] hover:text-white" on:click={clearHistory}>Clear all</button>
        {/if}
      </div>

      {#if $history.length === 0}
        <div class="pt-16" in:fade={{ duration: 150 }}>
          <p class="text-[20px] font-semibold">Nothing here yet.</p>
          <p class="mt-2 max-w-[440px] text-[14px] leading-relaxed text-[#8a8a8a]">
            Hold <span class="text-white">Ctrl + Win</span> in any app, speak, and let go. Your words are typed where
            your cursor is and saved here.
          </p>
        </div>
      {:else if groups.length === 0}
        <p class="pt-10 text-[14px] text-[#6b6b6b]">Nothing matches “{query}”.</p>
      {:else}
        {#each groups as group, groupIndex}
          <p class="pb-1 pt-6 text-[12px] font-medium text-[#5c5c5c]">{group.label}</p>
          {#each group.items as item, index (item.id)}
            {@const latest = groupIndex === 0 && index === 0}
            <div class="group flex items-start gap-4 py-2.5">
              <span class="mt-[9px] h-[5px] w-[5px] shrink-0 rounded-full {latest ? 'bg-[#ff5a4e]' : 'bg-[#333]'}"></span>
              <button class="min-w-0 flex-1 text-left" on:click={() => (expandedId = expandedId === item.id ? null : item.id)}>
                <p
                  class="text-[15px] leading-[1.55] {latest ? 'text-white' : 'text-[#c4c4c4]'} {expandedId === item.id
                    ? ''
                    : 'line-clamp-2'}"
                >
                  {item.text}
                </p>
                {#if expandedId === item.id}
                  <p class="mt-1.5 text-[12px] text-[#5c5c5c]" transition:fade={{ duration: 120 }}>
                    {item.language === 'uk' ? 'Ukrainian' : 'English'} · {item.latencyMs} ms · {wordCount(item.text)} words
                  </p>
                {/if}
              </button>
              <div class="relative w-28 shrink-0 pt-[3px] text-right text-[13px] tabular-nums">
                <span class="text-[#5c5c5c] group-hover:invisible">{formatTime(item.createdAt)}</span>
                <span class="invisible absolute right-0 top-[3px] flex gap-3 group-hover:visible">
                  <button class="text-[#a3a3a3] hover:text-white" on:click={() => copy(item.id, item.text)}>
                    {copiedId === item.id ? 'Copied' : 'Copy'}
                  </button>
                  <button class="text-[#a3a3a3] hover:text-[#ff5a4e]" on:click={() => deleteHistoryItem(item.id)}>Delete</button>
                </span>
              </div>
            </div>
          {/each}
        {/each}
      {/if}
    {:else if view === 'settings'}
      <div in:fade={{ duration: 150 }}>
        <h2 class="mt-8 border-b border-[#1a1a1a] pb-2 text-[15px] font-bold">Speech model</h2>
        {#each MODELS as model}
          <button
            class="flex w-full items-center gap-3.5 py-3 text-left disabled:opacity-50"
            disabled={$restarting}
            on:click={() => $settings.model !== model.id && patchSettings({ model: model.id })}
          >
            <span
              class="flex h-4 w-4 shrink-0 items-center justify-center rounded-full border {$settings.model === model.id
                ? 'border-white'
                : 'border-[#3a3a3a]'}"
            >
              {#if $settings.model === model.id}<span class="h-2 w-2 rounded-full bg-white"></span>{/if}
            </span>
            <span class="text-[14px] {$settings.model === model.id ? 'text-white' : 'text-[#a3a3a3]'}">{model.name}</span>
            <span class="text-[13px] text-[#5c5c5c]">{model.detail}</span>
            <span class="ml-auto text-[13px] tabular-nums text-[#5c5c5c]">{model.size}</span>
          </button>
        {/each}

        <div class="mt-2 flex items-center justify-between gap-6 py-3">
          <div>
            <p class="text-[14px]">Compute device</p>
            <p class="text-[13px] text-[#6b6b6b]">Auto uses the GPU when one is available.</p>
          </div>
          <div class="flex gap-4">
            {#each DEVICES as device}
              <button
                disabled={$restarting}
                class="text-[14px] underline-offset-[6px] transition-colors {$settings.device === device.id
                  ? 'text-white underline decoration-[1.5px]'
                  : 'text-[#525252] hover:text-[#a3a3a3]'}"
                on:click={() => $settings.device !== device.id && patchSettings({ device: device.id })}
              >
                {device.name}
              </button>
            {/each}
          </div>
        </div>

        {#if $settings.model === 'large_v3_turbo_q8'}
          <div class="flex items-center justify-between gap-6 py-3">
            <div class="min-w-0">
              <p class="text-[14px]">Languages</p>
              <p class="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-[13px] text-[#a3a3a3]">
                {#if $settings.autoDetectLanguage}
                  Auto-detect from all languages
                {:else}
                  {#each selected as language}
                    <span class="inline-flex items-center gap-1.5"><Flag {language} size={11} />{language.nativeName}</span>
                  {/each}
                {/if}
              </p>
            </div>
            <button class="shrink-0 text-[14px] text-white underline decoration-[#525252] underline-offset-[6px] hover:decoration-white" on:click={openLanguages}>
              Change
            </button>
          </div>
        {/if}

        <h2 class="mt-8 border-b border-[#1a1a1a] pb-2 text-[15px] font-bold">Startup</h2>
        {#each [{ key: LAUNCH, label: 'Launch at login', hint: 'Start OpenWhisper when you sign in to Windows.' }, { key: SHOW, label: 'Open this window on launch', hint: 'Off keeps OpenWhisper in the tray.' }] as row}
          {@const on = $settings[row.key]}
          <div class="flex items-center justify-between gap-6 py-3">
            <div>
              <p class="text-[14px]">{row.label}</p>
              <p class="text-[13px] text-[#6b6b6b]">{row.hint}</p>
            </div>
            <button
              role="switch"
              aria-checked={on}
              aria-label={row.label}
              class="relative h-5 w-9 shrink-0 rounded-full transition-colors {on ? 'bg-white' : 'bg-[#262626]'}"
              on:click={() => setStartupToggle(row.key, !on)}
            >
              <span class="absolute top-[3px] h-[14px] w-[14px] rounded-full transition-all {on ? 'left-[19px] bg-black' : 'left-[3px] bg-[#737373]'}"></span>
            </button>
          </div>
        {/each}

        <h2 class="mt-8 border-b border-[#1a1a1a] pb-2 text-[15px] font-bold">Engine</h2>
        <div class="flex items-center justify-between gap-6 py-3">
          <div>
            <p class="text-[14px]">Hold to dictate</p>
            <p class="text-[13px] text-[#6b6b6b]">Works in any app. Release to type the text.</p>
          </div>
          <span class="text-[14px] text-[#a3a3a3]">Ctrl + Win</span>
        </div>
        <div class="flex items-center justify-between gap-6 py-3">
          <div>
            <p class="text-[14px]">Speech engine</p>
            <p class="text-[13px] text-[#6b6b6b]">Runs locally. Restart it if dictation stops responding.</p>
          </div>
          <button
            class="text-[14px] text-white underline decoration-[#525252] underline-offset-[6px] hover:decoration-white disabled:animate-breathe disabled:no-underline"
            disabled={$restarting}
            on:click={restartEngine}
          >
            {$restarting ? 'Restarting…' : 'Restart'}
          </button>
        </div>

        <div class="mt-8 grid grid-cols-3 gap-6 border-t border-[#1a1a1a] pt-5">
          <div>
            <p class="text-[12px] text-[#6b6b6b]">Transcripts</p>
            <p class="mt-1 text-[16px] tabular-nums">{$stats.count}</p>
          </div>
          <div>
            <p class="text-[12px] text-[#6b6b6b]">Words dictated</p>
            <p class="mt-1 text-[16px] tabular-nums">{$stats.words}</p>
          </div>
          <div>
            <p class="text-[12px] text-[#6b6b6b]">Average latency</p>
            <p class="mt-1 text-[16px] tabular-nums">{$stats.avgLatencyMs ?? '—'} {$stats.avgLatencyMs ? 'ms' : ''}</p>
          </div>
        </div>
        <p class="mt-6 text-[12px] text-[#4d4d4d]">OpenWhisper 0.1.0. Audio never leaves this computer.</p>
      </div>
    {:else}
      <!-- Language picker as a full page, not a modal -->
      <div in:fade={{ duration: 150 }}>
        <div class="sticky top-0 z-10 flex items-center gap-6 bg-black pb-2 pt-6">
          <input
            bind:value={languageQuery}
            placeholder="Search 99 languages"
            class="flex-1 border-b border-[#1f1f1f] bg-transparent py-2 text-[14px] text-white placeholder:text-[#4d4d4d] focus:border-[#525252] focus:outline-none"
          />
          <button class="flex items-center gap-2.5 text-[14px]" on:click={() => (draft = { ...draft, auto: !draft.auto })}>
            <span class="relative h-5 w-9 rounded-full transition-colors {draft.auto ? 'bg-white' : 'bg-[#262626]'}">
              <span class="absolute top-[3px] h-[14px] w-[14px] rounded-full transition-all {draft.auto ? 'left-[19px] bg-black' : 'left-[3px] bg-[#737373]'}"></span>
            </span>
            Auto-detect
          </button>
        </div>
        <div class="grid grid-cols-2 gap-x-8 pt-3">
          {#each filteredLanguages as language (language.id)}
            {@const on = !draft.auto && draft.languages.includes(language.id)}
            <button
              class="flex items-center gap-3 border-b border-[#111] py-2.5 text-left"
              on:click={() => (draft = toggleDraftLanguage(draft, language.id))}
            >
              <Flag {language} size={12} />
              <span class="text-[14px] {on ? 'text-white' : 'text-[#8a8a8a]'}">{language.name}</span>
              {#if language.nativeName !== language.name}
                <span class="truncate text-[13px] text-[#4d4d4d]">{language.nativeName}</span>
              {/if}
              <span class="ml-auto text-[13px] {on ? 'text-white' : 'text-transparent'}">✓</span>
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </div>

  {#if view === 'languages'}
    <footer class="flex shrink-0 items-center justify-end gap-6 border-t border-[#1a1a1a] px-8 py-4">
      <button class="text-[14px] text-[#8a8a8a] hover:text-white" on:click={() => (view = 'settings')}>Cancel</button>
      <button
        class="rounded-full bg-white px-5 py-2 text-[14px] font-semibold text-black hover:bg-[#e5e5e5]"
        on:click={() => {
          saveDraft(draft);
          view = 'settings';
        }}
      >
        Save
      </button>
    </footer>
  {/if}

  <!-- Toasts -->
  <div class="pointer-events-none absolute left-8 right-8 z-50 flex flex-col gap-2 {view === 'languages' ? 'bottom-[84px]' : 'bottom-5'}">
    {#each $toasts as toast (toast.id)}
      <div
        in:fly={{ y: 12, duration: 200 }}
        out:fade={{ duration: 150 }}
        class="pointer-events-auto flex items-start gap-3 rounded-2xl border border-[#262626] bg-black px-5 py-4"
      >
        <p class="flex-1 text-[14px] leading-relaxed text-[#c4c4c4]">
          <b class="font-bold text-white">Needs attention.</b>
          {toast.message}
          <span class="ml-1 text-[12px] text-[#4d4d4d]">{toast.code}</span>
        </p>
        <button class="text-[13px] text-[#6b6b6b] hover:text-white" on:click={() => dismissToast(toast.id)}>Dismiss</button>
      </div>
    {/each}
  </div>
</div>
