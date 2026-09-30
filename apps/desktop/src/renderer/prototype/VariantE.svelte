<script lang="ts">
  // PROTOTYPE variant E, "Ledger Split": D's two-pane structure written in A's voice.
  // A's title-as-navigation and status sentence span the top; below, a list pane and a
  // detail pane. Near-black blue-tinted surfaces, blue for selection, cyan for "live".
  import { fade, fly } from 'svelte/transition';
  import Flag from './Flag.svelte';
  import WindowControls from './WindowControls.svelte';
  import {
    DEVICES,
    MODELS,
    clearHistory,
    copyText,
    dayLabel,
    deleteHistoryItem,
    dismissToast,
    draftFromSettings,
    filterLanguages,
    formatTime,
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
    { id: 'engine', name: 'Speech engine' },
    { id: 'languages', name: 'Languages' },
    { id: 'general', name: 'General' },
    { id: 'about', name: 'About' },
  ];

  let view: View = 'history';
  let section: Section = 'engine';
  let query = '';
  let selectedId: string | null = null;
  let copied = false;
  let draft: LanguageDraft = draftFromSettings($settings);
  let pinnedLanguages: LanguageDraft['languages'] = [];
  let languageQuery = '';

  $: needle = query.trim().toLowerCase();
  $: rows = needle ? $history.filter((item) => item.text.toLowerCase().includes(needle)) : $history;
  $: selected = rows.find((item) => item.id === selectedId) ?? rows[0];
  $: latestId = $history[0]?.id;
  $: spoken = languageOptions($settings.spokenLanguages);
  $: modelName = MODELS.find((model) => model.id === $settings.model)?.name ?? '';
  $: deviceName = $settings.device === 'auto' ? 'the fastest device' : $settings.device.toUpperCase();
  $: dirty =
    draft.auto !== $settings.autoDetectLanguage || draft.languages.join() !== $settings.spokenLanguages.join();

  async function copySelected() {
    if (!selected || !(await copyText(selected.text))) return;
    copied = true;
    setTimeout(() => (copied = false), 1400);
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
  <header class="drag flex h-11 shrink-0 items-center justify-between pl-7">
    <span class="text-[12px] font-medium text-[#5d6773]">OpenWhisper</span>
    <WindowControls color="#9aa4b0" />
  </header>

  <!-- A: title-as-navigation + status sentence -->
  <div class="shrink-0 px-7 pb-5 pt-2">
    <nav class="flex items-baseline gap-5">
      {#each ['history', 'settings'] as item}
        <button
          class="text-[28px] font-semibold capitalize tracking-[-0.02em] transition-colors {view === item
            ? 'text-white'
            : 'text-[#2c333d] hover:text-[#5d6773]'}"
          on:click={() => (view = item === 'history' ? 'history' : 'settings')}
        >
          {item}
        </button>
      {/each}
    </nav>
    <p class="mt-1.5 flex items-center gap-2 text-[13px] text-[#9aa4b0]">
      <span class="h-1.5 w-1.5 rounded-full {$restarting ? 'animate-breathe bg-[#5d6773]' : 'bg-[#22d3ee] shadow-[0_0_8px_#22d3ee]'}"></span>
      {#if $restarting}
        <span><b class="font-semibold text-white">Restarting.</b> The speech engine is reloading.</span>
      {:else}
        <span>
          <b class="font-semibold text-white">Ready.</b> Hold <span class="text-[#7dd3fc]">Ctrl + Win</span> anywhere to dictate.
          {modelName} on {deviceName}, {$settings.autoDetectLanguage ? 'any language' : spoken.map((language) => language.nativeName).join(', ')}.
        </span>
      {/if}
    </p>
  </div>

  <div class="flex min-h-0 flex-1 border-t border-[#1b2027]">
    <!-- Left pane -->
    <aside class="flex w-[310px] shrink-0 flex-col border-r border-[#1b2027] bg-[#0f1216]">
      {#if view === 'history'}
        <div class="flex items-center gap-3 px-5 pt-3">
          <input
            bind:value={query}
            placeholder="Search"
            class="flex-1 border-b border-[#1b2027] bg-transparent py-2 text-[13px] text-white placeholder:text-[#4a535e] focus:border-[#3b82f6] focus:outline-none"
          />
          {#if $history.length > 0}
            <button class="text-[12px] text-[#5d6773] hover:text-white" on:click={clearHistory}>Clear</button>
          {/if}
        </div>
        <div class="scroll-area min-h-0 flex-1 overflow-y-auto pb-3">
          {#if $history.length === 0}
            <p class="px-5 py-6 text-[13px] text-[#5d6773]">Nothing here yet.</p>
          {:else if rows.length === 0}
            <p class="px-5 py-6 text-[13px] text-[#5d6773]">Nothing matches “{query}”.</p>
          {/if}
          {#each rows as item, index (item.id)}
            {@const active = selected?.id === item.id}
            {#if index === 0 || dayLabel(rows[index - 1].createdAt) !== dayLabel(item.createdAt)}
              <p class="px-5 pb-1 pt-4 text-[11px] font-medium text-[#4a535e]">{dayLabel(item.createdAt)}</p>
            {/if}
            <button
              class="relative flex w-full items-start gap-3 px-5 py-2.5 text-left transition-colors {active ? 'bg-[#151a20]' : 'hover:bg-[#12161b]'}"
              on:click={() => (selectedId = item.id)}
            >
              {#if active}<span class="absolute inset-y-0 left-0 w-[2px]" style="background: {GRADIENT}"></span>{/if}
              <span class="mt-[7px] h-[5px] w-[5px] shrink-0 rounded-full {item.id === latestId ? 'bg-[#22d3ee]' : active ? 'bg-[#3b82f6]' : 'bg-[#2c333d]'}"></span>
              <span class="line-clamp-2 min-w-0 flex-1 text-[13px] leading-[1.5] {active ? 'text-white' : 'text-[#aab3bd]'}">{item.text}</span>
              <span class="shrink-0 pt-px text-[11px] tabular-nums text-[#4a535e]">{formatTime(item.createdAt)}</span>
            </button>
          {/each}
        </div>
      {:else}
        <div class="py-3">
          {#each SECTIONS as item}
            <button
              class="relative flex w-full items-center gap-3 px-5 py-2.5 text-left text-[14px] transition-colors {section === item.id
                ? 'bg-[#151a20] text-white'
                : 'text-[#7c8792] hover:bg-[#12161b] hover:text-[#c5cdd6]'}"
              on:click={() => openSection(item.id)}
            >
              {#if section === item.id}<span class="absolute inset-y-0 left-0 w-[2px]" style="background: {GRADIENT}"></span>{/if}
              {item.name}
            </button>
          {/each}
        </div>
      {/if}
    </aside>

    <!-- Right pane -->
    <main class="scroll-area relative min-w-0 flex-1 overflow-y-auto">
      {#if view === 'history'}
        {#if !selected}
          <div class="flex h-full flex-col items-center justify-center gap-5 px-10 text-center" in:fade={{ duration: 150 }}>
            <div class="flex items-center gap-2">
              {#each ['Ctrl', 'Win'] as key}
                <kbd class="rounded-xl border border-[#1f3b5c] border-b-[3px] border-b-[#1e4f7a] bg-[#0f1822] px-5 py-3 font-sans text-[18px] font-semibold text-[#7dd3fc] shadow-[0_0_24px_rgba(34,211,238,0.12)]">{key}</kbd>
              {/each}
            </div>
            <div>
              <p class="text-[18px] font-semibold">Nothing here yet.</p>
              <p class="mt-1.5 max-w-[320px] text-[13px] leading-relaxed text-[#7c8792]">
                Hold both keys in any app, speak, and let go. Your words are typed at your cursor and saved here.
              </p>
            </div>
          </div>
        {:else}
          {#key selected.id}
            <article class="px-8 py-6" in:fade={{ duration: 120 }}>
              <div class="flex items-center gap-2">
                <p class="text-[13px] text-[#5d6773]">{dayLabel(selected.createdAt)} · {formatTime(selected.createdAt)}</p>
                <span class="flex-1"></span>
                <button
                  class="rounded-lg px-3.5 py-1.5 text-[13px] font-semibold transition-colors {copied ? 'bg-[#0e3a44] text-[#67e8f9]' : 'bg-[#3b82f6] text-white hover:bg-[#4b8ef8]'}"
                  on:click={copySelected}
                >
                  {copied ? 'Copied' : 'Copy'}
                </button>
                <button
                  class="rounded-lg px-3 py-1.5 text-[13px] text-[#7c8792] hover:bg-[#151a20] hover:text-[#fb7185]"
                  on:click={() => deleteHistoryItem(selected.id)}
                >
                  Delete
                </button>
              </div>
              <p class="mt-5 whitespace-pre-wrap text-[18px] leading-[1.65] text-white">{selected.text}</p>
              <div class="mt-8 grid grid-cols-4 gap-6 border-t border-[#1b2027] pt-4">
                {#each [{ label: 'Language', value: selected.language === 'uk' ? 'Ukrainian' : 'English' }, { label: 'Latency', value: `${selected.latencyMs} ms` }, { label: 'Words', value: String(wordCount(selected.text)) }, { label: 'Characters', value: String(selected.text.length) }] as cell}
                  <div>
                    <p class="text-[11px] text-[#5d6773]">{cell.label}</p>
                    <p class="mt-1 text-[15px] tabular-nums">{cell.value}</p>
                  </div>
                {/each}
              </div>
            </article>
          {/key}
        {/if}
      {:else}
        <!-- A: settings as a document, one section at a time -->
        <div class="px-8 py-6" in:fade={{ duration: 120 }}>
          {#if section === 'engine'}
            <h2 class="border-b border-[#1b2027] pb-2 text-[15px] font-bold">Speech model</h2>
            {#each MODELS as model}
              {@const on = $settings.model === model.id}
              <button
                disabled={$restarting}
                class="flex w-full items-center gap-3.5 py-3 text-left disabled:opacity-50"
                on:click={() => !on && patchSettings({ model: model.id })}
              >
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
              <div class="inline-flex rounded-lg bg-[#0f1216] p-0.5 ring-1 ring-[#1b2027]">
                {#each DEVICES as device}
                  <button
                    disabled={$restarting}
                    class="rounded-md px-4 py-1.5 text-[13px] transition-colors {$settings.device === device.id ? 'bg-[#172338] text-[#93c5fd]' : 'text-[#5d6773] hover:text-[#c5cdd6]'}"
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
                class="text-[14px] text-[#7dd3fc] underline decoration-[#1e4f7a] underline-offset-[6px] hover:decoration-[#7dd3fc] disabled:animate-breathe disabled:no-underline"
                on:click={restartEngine}
              >
                {$restarting ? 'Restarting…' : 'Restart'}
              </button>
            </div>
          {:else if section === 'languages'}
            <h2 class="border-b border-[#1b2027] pb-2 text-[15px] font-bold">Languages you speak</h2>
            {#if $settings.model === 'medium_en_q8'}
              <p class="mt-3 text-[13px] text-[#7c8792]">The English-only model only understands English. Switch to Multilingual to pick languages.</p>
            {:else}
              <div class="flex items-center gap-5 pt-3">
                <input
                  bind:value={languageQuery}
                  placeholder="Search 99 languages"
                  class="flex-1 border-b border-[#1b2027] bg-transparent py-2 text-[13px] text-white placeholder:text-[#4a535e] focus:border-[#3b82f6] focus:outline-none"
                />
                <button class="flex items-center gap-2.5 text-[13px]" on:click={() => (draft = { ...draft, auto: !draft.auto })}>
                  <span class="relative h-5 w-9 rounded-full transition-colors" style="background: {draft.auto ? GRADIENT : '#1b2027'}">
                    <span class="absolute top-[3px] h-[14px] w-[14px] rounded-full bg-white transition-all {draft.auto ? 'left-[19px]' : 'left-[3px]'}"></span>
                  </span>
                  Auto-detect
                </button>
              </div>
              <div class="pt-2 {draft.auto ? 'opacity-40' : ''} {dirty ? 'pb-16' : ''}">
                {#each filterLanguages(languageQuery, pinnedLanguages) as language (language.id)}
                  {@const on = !draft.auto && draft.languages.includes(language.id)}
                  <button
                    class="flex w-full items-center gap-3 border-b border-[#12161b] py-2.5 text-left"
                    on:click={() => (draft = toggleDraftLanguage(draft, language.id))}
                  >
                    <Flag {language} size={12} />
                    <span class="text-[14px] {on ? 'text-white' : 'text-[#7c8792]'}">{language.name}</span>
                    {#if language.nativeName !== language.name}<span class="text-[13px] text-[#4a535e]">{language.nativeName}</span>{/if}
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
            <h2 class="border-b border-[#1b2027] pb-2 text-[15px] font-bold">About</h2>
            <div class="mt-4 grid grid-cols-3 gap-6">
              {#each [{ label: 'Transcripts', value: String($stats.count) }, { label: 'Words dictated', value: String($stats.words) }, { label: 'Average latency', value: $stats.avgLatencyMs ? `${$stats.avgLatencyMs} ms` : '—' }] as cell}
                <div>
                  <p class="text-[12px] text-[#5d6773]">{cell.label}</p>
                  <p class="mt-1 text-[22px] font-semibold tabular-nums">{cell.value}</p>
                </div>
              {/each}
            </div>
            <p class="mt-7 text-[13px] text-[#7c8792]">OpenWhisper 0.1.0</p>
            <p class="text-[13px] text-[#4a535e]">Fully offline. Audio never leaves this computer.</p>
          {/if}
        </div>

        {#if section === 'languages' && dirty}
          <div class="absolute inset-x-0 bottom-0 flex items-center gap-4 border-t border-[#1b2027] bg-[#0b0d10]/95 px-8 py-3" transition:fly={{ y: 20, duration: 180 }}>
            <p class="flex-1 text-[13px] text-[#9aa4b0]">
              <b class="font-semibold text-white">{draft.auto ? 'Auto-detect.' : `${draft.languages.length} selected.`}</b> Applying restarts the engine.
            </p>
            <button class="text-[13px] text-[#7c8792] hover:text-white" on:click={() => openSection('languages')}>Discard</button>
            <button class="rounded-lg bg-[#3b82f6] px-4 py-1.5 text-[13px] font-semibold text-white hover:bg-[#4b8ef8]" on:click={() => saveDraft(draft)}>Apply</button>
          </div>
        {/if}
      {/if}
    </main>
  </div>

  <!-- Toasts: A's voice, D's actionable card -->
  <div class="pointer-events-none absolute right-5 top-14 z-50 flex w-[340px] flex-col gap-2">
    {#each $toasts as toast (toast.id)}
      <div
        in:fly={{ y: -10, duration: 200 }}
        out:fade={{ duration: 150 }}
        class="pointer-events-auto rounded-xl border border-[#1b2027] bg-[#11151a] p-4 shadow-[0_16px_50px_rgba(0,0,0,0.6)]"
      >
        <p class="text-[13px] leading-relaxed text-[#aab3bd]"><b class="font-bold text-white">Needs attention.</b> {toast.message}</p>
        <div class="mt-3 flex gap-2">
          <button
            class="rounded-lg bg-[#3b82f6] px-3 py-1 text-[12px] font-semibold text-white"
            on:click={() => {
              restartEngine();
              dismissToast(toast.id);
            }}
          >
            Restart engine
          </button>
          <button class="rounded-lg px-3 py-1 text-[12px] text-[#7c8792] hover:text-white" on:click={() => dismissToast(toast.id)}>Dismiss</button>
        </div>
      </div>
    {/each}
  </div>
</div>
