<script lang="ts">
  // PROTOTYPE variant D, "Split": a mail-client layout. The left pane lists transcripts or
  // settings sections; the right pane shows the selected item. No modals anywhere; the
  // language picker is an inline pane with an apply bar.
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
    dayLabel,
    history,
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

  type Section = 'engine' | 'languages' | 'general' | 'about';

  const BLUE = '#4c8dff';
  const SECTIONS: Array<{ id: Section; name: string; hint: string }> = [
    { id: 'engine', name: 'Speech engine', hint: 'Model and device' },
    { id: 'languages', name: 'Languages', hint: 'What you speak' },
    { id: 'general', name: 'General', hint: 'Startup and shortcut' },
    { id: 'about', name: 'About', hint: 'Version and stats' },
  ];

  let mode: 'transcripts' | 'settings' = 'transcripts';
  let section: Section = 'engine';
  let query = '';
  let selectedId: string | null = null;
  let copied = false;
  let draft: LanguageDraft = draftFromSettings($settings);
  let languageQuery = '';

  $: needle = query.trim().toLowerCase();
  $: rows = needle ? $history.filter((item) => item.text.toLowerCase().includes(needle)) : $history;
  $: selected = rows.find((item) => item.id === selectedId) ?? rows[0];
  $: dirty =
    draft.auto !== $settings.autoDetectLanguage ||
    draft.languages.join() !== $settings.spokenLanguages.join();
  $: languageRows = filterLanguages(languageQuery);
  $: pinned = draft.auto ? [] : languageRows.filter((language) => draft.languages.includes(language.id));
  $: rest = languageRows.filter((language) => draft.auto || !draft.languages.includes(language.id));

  async function copySelected() {
    if (!selected || !(await copyText(selected.text))) return;
    copied = true;
    setTimeout(() => (copied = false), 1400);
  }

  function openSection(id: Section) {
    section = id;
    if (id === 'languages') {
      draft = draftFromSettings($settings);
      languageQuery = '';
    }
  }
</script>

<div class="relative flex h-full flex-col bg-black text-[#ededed]" style="font-family: Geist, sans-serif;">
  <header class="drag flex h-11 shrink-0 items-center border-b border-[#141414] pl-4">
    <span class="mr-2.5 flex h-5 w-5 items-center justify-center rounded-md" style="background: {BLUE}">
      <svg viewBox="0 0 24 24" class="h-3 w-3 text-black" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round"><path d="M6 10v4M10 7v10M14 5v14M18 9v6" /></svg>
    </span>
    <span class="text-[13px] font-semibold">OpenWhisper</span>
    <span class="flex-1"></span>
    <WindowControls color="#8a8a8a" />
  </header>

  <div class="flex min-h-0 flex-1">
    <!-- Left pane -->
    <aside class="flex w-[300px] shrink-0 flex-col border-r border-[#141414]">
      <div class="p-3">
        <div class="grid grid-cols-2 rounded-lg bg-[#0c0c0c] p-0.5 text-[12px] font-medium">
          {#each ['transcripts', 'settings'] as item}
            <button
              class="rounded-md py-1.5 capitalize transition-colors {mode === item ? 'bg-[#1c1c1c] text-white' : 'text-[#6b6b6b] hover:text-[#bdbdbd]'}"
              on:click={() => (mode = item === 'transcripts' ? 'transcripts' : 'settings')}
            >
              {item}{item === 'transcripts' && $stats.count ? ` · ${$stats.count}` : ''}
            </button>
          {/each}
        </div>
        {#if mode === 'transcripts'}
          <div class="relative mt-2.5">
            <svg viewBox="0 0 24 24" class="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-[#4d4d4d]" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>
            <input
              bind:value={query}
              placeholder="Search"
              class="w-full rounded-lg border border-[#1a1a1a] bg-black py-1.5 pl-8 pr-3 text-[13px] text-white placeholder:text-[#4d4d4d] focus:border-[#333] focus:outline-none"
            />
          </div>
        {/if}
      </div>

      <div class="scroll-area min-h-0 flex-1 overflow-y-auto">
        {#if mode === 'transcripts'}
          {#if $history.length === 0}
            <p class="px-4 py-6 text-[13px] text-[#5c5c5c]">No transcripts yet.</p>
          {:else if rows.length === 0}
            <p class="px-4 py-6 text-[13px] text-[#5c5c5c]">No results.</p>
          {/if}
          {#each rows as item, index (item.id)}
            {#if index === 0 || dayLabel(rows[index - 1].createdAt) !== dayLabel(item.createdAt)}
              <p class="px-4 pb-1 pt-3 text-[11px] font-medium text-[#4d4d4d]">{dayLabel(item.createdAt)}</p>
            {/if}
            <button
              class="relative block w-full px-4 py-2.5 text-left transition-colors {selected?.id === item.id ? 'bg-[#0e0e0e]' : 'hover:bg-[#080808]'}"
              on:click={() => (selectedId = item.id)}
            >
              {#if selected?.id === item.id}<span class="absolute inset-y-2 left-0 w-[2px] rounded-full" style="background: {BLUE}"></span>{/if}
              <span class="flex items-center gap-2 text-[11px] text-[#5c5c5c]">
                <span class="tabular-nums">{formatTime(item.createdAt)}</span>
                <span class="rounded bg-[#141414] px-1 py-px font-medium uppercase text-[#8a8a8a]">{item.language}</span>
              </span>
              <span class="mt-1 line-clamp-2 text-[13px] leading-snug {selected?.id === item.id ? 'text-white' : 'text-[#a3a3a3]'}">{item.text}</span>
            </button>
          {/each}
        {:else}
          {#each SECTIONS as item}
            <button
              class="relative block w-full px-4 py-3 text-left transition-colors {section === item.id ? 'bg-[#0e0e0e]' : 'hover:bg-[#080808]'}"
              on:click={() => openSection(item.id)}
            >
              {#if section === item.id}<span class="absolute inset-y-2 left-0 w-[2px] rounded-full" style="background: {BLUE}"></span>{/if}
              <span class="block text-[13px] {section === item.id ? 'text-white' : 'text-[#bdbdbd]'}">{item.name}</span>
              <span class="block text-[12px] text-[#5c5c5c]">{item.hint}</span>
            </button>
          {/each}
        {/if}
      </div>

      <!-- Engine status, always visible -->
      <div class="flex items-center gap-2.5 border-t border-[#141414] px-4 py-3 text-[12px]">
        <span class="h-2 w-2 rounded-full {$restarting ? 'animate-breathe bg-[#6b6b6b]' : 'bg-[#34d399]'}"></span>
        <span class="text-[#a3a3a3]">{$restarting ? 'Restarting…' : 'Ready'}</span>
        <span class="ml-auto flex gap-1">
          {#each ['Ctrl', 'Win'] as key}
            <kbd class="rounded border border-[#262626] border-b-[#333] bg-[#0a0a0a] px-1.5 py-px font-sans text-[11px] text-[#8a8a8a]">{key}</kbd>
          {/each}
        </span>
      </div>
    </aside>

    <!-- Right pane -->
    <main class="scroll-area relative min-w-0 flex-1 overflow-y-auto">
      {#if mode === 'transcripts'}
        {#if !selected}
          <div class="flex h-full flex-col items-center justify-center gap-5 text-center" in:fade={{ duration: 150 }}>
            <div class="flex items-center gap-2">
              {#each ['Ctrl', 'Win'] as key}
                <kbd class="rounded-xl border border-[#262626] border-b-[3px] border-b-[#333] bg-[#0a0a0a] px-5 py-3 font-sans text-[18px] font-medium text-white">{key}</kbd>
              {/each}
            </div>
            <div>
              <p class="text-[15px] font-medium">Hold to dictate, anywhere</p>
              <p class="mt-1 text-[13px] text-[#6b6b6b]">Release and the text is typed at your cursor.</p>
            </div>
          </div>
        {:else}
          {#key selected.id}
            <article class="px-8 py-7" in:fade={{ duration: 120 }}>
              <div class="flex items-center gap-2">
                <p class="text-[13px] text-[#6b6b6b]">{dayLabel(selected.createdAt)} · {formatTime(selected.createdAt)}</p>
                <span class="flex-1"></span>
                <button
                  class="rounded-lg px-3.5 py-1.5 text-[13px] font-medium text-black transition-opacity hover:opacity-90"
                  style="background: {copied ? '#34d399' : BLUE}"
                  on:click={copySelected}
                >
                  {copied ? 'Copied' : 'Copy'}
                </button>
                <button
                  class="rounded-lg border border-[#1f1f1f] p-2 text-[#8a8a8a] hover:border-[#3a1f24] hover:text-[#ff6b6b]"
                  aria-label="Delete"
                  on:click={() => deleteHistoryItem(selected.id)}
                >
                  <svg viewBox="0 0 24 24" class="h-3.5 w-3.5" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13" /></svg>
                </button>
              </div>
              <p class="mt-5 whitespace-pre-wrap text-[19px] leading-[1.6] text-white">{selected.text}</p>
              <div class="mt-8 grid grid-cols-4 overflow-hidden rounded-xl border border-[#161616]">
                {#each [{ label: 'Language', value: selected.language === 'uk' ? 'Ukrainian' : 'English' }, { label: 'Latency', value: `${selected.latencyMs} ms` }, { label: 'Words', value: String(wordCount(selected.text)) }, { label: 'Characters', value: String(selected.text.length) }] as cell, index}
                  <div class="px-4 py-3 {index > 0 ? 'border-l border-[#161616]' : ''}">
                    <p class="text-[11px] text-[#5c5c5c]">{cell.label}</p>
                    <p class="mt-0.5 text-[14px] tabular-nums">{cell.value}</p>
                  </div>
                {/each}
              </div>
            </article>
          {/key}
        {/if}
      {:else}
        <div class="px-8 py-7" in:fade={{ duration: 120 }}>
          <h1 class="text-[20px] font-semibold">{SECTIONS.find((item) => item.id === section)?.name}</h1>

          {#if section === 'engine'}
            <p class="mt-1 text-[13px] text-[#6b6b6b]">Changes restart the engine. It takes a few seconds.</p>
            <div class="mt-6 space-y-2">
              {#each MODELS as model}
                {@const on = $settings.model === model.id}
                <button
                  disabled={$restarting}
                  class="flex w-full items-center gap-3.5 rounded-xl border px-4 py-3.5 text-left transition-colors disabled:opacity-50 {on ? '' : 'border-[#1a1a1a] hover:border-[#2a2a2a]'}"
                  style={on ? `border-color: ${BLUE}; background: rgba(76,141,255,0.06)` : ''}
                  on:click={() => !on && patchSettings({ model: model.id })}
                >
                  <span class="flex h-4 w-4 items-center justify-center rounded-full border-2" style="border-color: {on ? BLUE : '#333'}">
                    {#if on}<span class="h-1.5 w-1.5 rounded-full" style="background: {BLUE}"></span>{/if}
                  </span>
                  <span class="flex-1">
                    <span class="block text-[14px] font-medium">{model.name}</span>
                    <span class="block text-[12px] text-[#6b6b6b]">{model.detail}</span>
                  </span>
                  <span class="text-[12px] tabular-nums text-[#5c5c5c]">{model.size}</span>
                </button>
              {/each}
            </div>
            <p class="mb-2 mt-7 text-[13px] font-medium">Compute device</p>
            <div class="inline-flex rounded-lg bg-[#0c0c0c] p-0.5">
              {#each DEVICES as device}
                <button
                  disabled={$restarting}
                  class="rounded-md px-5 py-1.5 text-[13px] transition-colors {$settings.device === device.id ? 'bg-[#1c1c1c] text-white' : 'text-[#6b6b6b] hover:text-[#bdbdbd]'}"
                  on:click={() => $settings.device !== device.id && patchSettings({ device: device.id })}
                >
                  {device.name}
                </button>
              {/each}
            </div>
            <div class="mt-7 flex items-center justify-between rounded-xl border border-[#161616] px-4 py-3.5">
              <div>
                <p class="text-[13px] font-medium">Engine not responding?</p>
                <p class="text-[12px] text-[#6b6b6b]">Restart the local transcription process.</p>
              </div>
              <button disabled={$restarting} class="rounded-lg border border-[#262626] px-3.5 py-1.5 text-[13px] hover:bg-[#111] disabled:opacity-50" on:click={restartEngine}>
                {$restarting ? 'Restarting…' : 'Restart'}
              </button>
            </div>
          {:else if section === 'languages'}
            {#if $settings.model === 'medium_en_q8'}
              <p class="mt-1 text-[13px] text-[#6b6b6b]">The English-only model only understands English. Switch to Multilingual to pick languages.</p>
            {:else}
              <p class="mt-1 text-[13px] text-[#6b6b6b]">Fewer languages means faster, more accurate results.</p>
              <button class="mt-5 flex w-full items-center justify-between rounded-xl border border-[#161616] px-4 py-3 text-left" on:click={() => (draft = { ...draft, auto: !draft.auto })}>
                <span>
                  <span class="block text-[13px] font-medium">Detect automatically</span>
                  <span class="block text-[12px] text-[#6b6b6b]">Listen for every supported language.</span>
                </span>
                <span class="relative h-5 w-9 rounded-full transition-colors" style="background: {draft.auto ? BLUE : '#222'}">
                  <span class="absolute top-[3px] h-[14px] w-[14px] rounded-full bg-white transition-all {draft.auto ? 'left-[19px]' : 'left-[3px]'}"></span>
                </span>
              </button>
              <input
                bind:value={languageQuery}
                placeholder="Filter languages"
                class="mt-4 w-full rounded-lg border border-[#1a1a1a] bg-black px-3 py-1.5 text-[13px] text-white placeholder:text-[#4d4d4d] focus:border-[#333] focus:outline-none"
              />
              <div class="mt-2 {draft.auto ? 'opacity-40' : ''}">
                {#each [...pinned, ...rest] as language, index (language.id)}
                  {@const on = !draft.auto && draft.languages.includes(language.id)}
                  {#if index === pinned.length && pinned.length > 0}<div class="my-1 h-px bg-[#161616]"></div>{/if}
                  <button class="flex w-full items-center gap-3 rounded-lg px-2 py-1.5 text-left hover:bg-[#0a0a0a]" on:click={() => (draft = toggleDraftLanguage(draft, language.id))}>
                    <span class="flex h-4 w-4 items-center justify-center rounded border" style="border-color: {on ? BLUE : '#333'}; background: {on ? BLUE : 'transparent'}">
                      {#if on}<svg viewBox="0 0 24 24" class="h-3 w-3 text-black" fill="none" stroke="currentColor" stroke-width="3.5" stroke-linecap="round"><path d="M5 12l5 5 9-10" /></svg>{/if}
                    </span>
                    <Flag {language} size={12} />
                    <span class="text-[13px] {on ? 'text-white' : 'text-[#bdbdbd]'}">{language.name}</span>
                    {#if language.nativeName !== language.name}<span class="text-[12px] text-[#5c5c5c]">{language.nativeName}</span>{/if}
                  </button>
                {/each}
              </div>
            {/if}
          {:else if section === 'general'}
            <div class="mt-6 divide-y divide-[#141414] rounded-xl border border-[#161616]">
              {#each [{ label: 'Launch at login', hint: 'Start with Windows, in the tray.', key: LAUNCH }, { label: 'Open window on launch', hint: 'Off keeps it tray-only.', key: SHOW }] as row}
                {@const on = $settings[row.key]}
                <button class="flex w-full items-center justify-between px-4 py-3.5 text-left" on:click={() => setStartupToggle(row.key, !on)}>
                  <span>
                    <span class="block text-[13px] font-medium">{row.label}</span>
                    <span class="block text-[12px] text-[#6b6b6b]">{row.hint}</span>
                  </span>
                  <span class="relative h-5 w-9 rounded-full transition-colors" style="background: {on ? BLUE : '#222'}">
                    <span class="absolute top-[3px] h-[14px] w-[14px] rounded-full bg-white transition-all {on ? 'left-[19px]' : 'left-[3px]'}"></span>
                  </span>
                </button>
              {/each}
              <div class="flex items-center justify-between px-4 py-3.5">
                <span>
                  <span class="block text-[13px] font-medium">Dictation shortcut</span>
                  <span class="block text-[12px] text-[#6b6b6b]">Hold to record, release to type.</span>
                </span>
                <span class="flex gap-1">
                  {#each ['Ctrl', 'Win'] as key}
                    <kbd class="rounded-md border border-[#262626] border-b-2 border-b-[#333] bg-[#0a0a0a] px-2 py-0.5 font-sans text-[12px] text-[#bdbdbd]">{key}</kbd>
                  {/each}
                </span>
              </div>
            </div>
            <div class="mt-6 flex items-center justify-between rounded-xl border border-[#2a1519] px-4 py-3.5">
              <span>
                <span class="block text-[13px] font-medium">Clear history</span>
                <span class="block text-[12px] text-[#6b6b6b]">Deletes all {$stats.count} saved transcripts.</span>
              </span>
              <button disabled={$stats.count === 0} class="rounded-lg px-3.5 py-1.5 text-[13px] text-[#ff6b6b] hover:bg-[#1a0c0e] disabled:opacity-40" on:click={clearHistory}>Clear</button>
            </div>
          {:else}
            <div class="mt-6 grid grid-cols-3 overflow-hidden rounded-xl border border-[#161616]">
              {#each [{ label: 'Transcripts', value: String($stats.count) }, { label: 'Words', value: String($stats.words) }, { label: 'Avg latency', value: $stats.avgLatencyMs ? `${$stats.avgLatencyMs} ms` : '—' }] as cell, index}
                <div class="px-4 py-4 {index > 0 ? 'border-l border-[#161616]' : ''}">
                  <p class="text-[11px] text-[#5c5c5c]">{cell.label}</p>
                  <p class="mt-1 text-[22px] font-semibold tabular-nums">{cell.value}</p>
                </div>
              {/each}
            </div>
            <p class="mt-6 text-[13px] text-[#8a8a8a]">OpenWhisper 0.1.0</p>
            <p class="text-[13px] text-[#5c5c5c]">Fully offline speech-to-text. Your audio never leaves this computer.</p>
          {/if}
        </div>

        {#if section === 'languages' && dirty}
          <div class="sticky bottom-0 flex items-center gap-3 border-t border-[#161616] bg-black/95 px-8 py-3" transition:fly={{ y: 20, duration: 180 }}>
            <p class="flex-1 text-[12px] text-[#8a8a8a]">
              {draft.auto ? 'Auto-detect' : `${draft.languages.length} language${draft.languages.length === 1 ? '' : 's'}`} · applying restarts the engine
            </p>
            <button class="rounded-lg px-3 py-1.5 text-[13px] text-[#8a8a8a] hover:text-white" on:click={() => (draft = draftFromSettings($settings))}>Discard</button>
            <button class="rounded-lg px-3.5 py-1.5 text-[13px] font-medium text-black" style="background: {BLUE}" on:click={() => saveDraft(draft)}>Apply</button>
          </div>
        {/if}
      {/if}
    </main>
  </div>

  <!-- Toasts -->
  <div class="pointer-events-none absolute right-4 top-14 z-50 flex w-[320px] flex-col gap-2">
    {#each $toasts as toast (toast.id)}
      <div
        in:fly={{ y: -10, duration: 200 }}
        out:fade={{ duration: 150 }}
        class="pointer-events-auto overflow-hidden rounded-xl border border-[#1f1f1f] bg-[#0a0a0a] shadow-[0_16px_50px_rgba(0,0,0,0.8)]"
      >
        <div class="flex gap-3 p-3.5">
          <span class="mt-1 h-2 w-2 shrink-0 rounded-full bg-[#ff6b6b]"></span>
          <div class="min-w-0 flex-1">
            <p class="text-[13px] font-medium">Engine disconnected</p>
            <p class="mt-0.5 text-[12px] leading-snug text-[#8a8a8a]">{toast.message}</p>
            <div class="mt-2.5 flex gap-2">
              <button
                class="rounded-md px-2.5 py-1 text-[12px] font-medium text-black"
                style="background: {BLUE}"
                on:click={() => {
                  restartEngine();
                  dismissToast(toast.id);
                }}
              >
                Restart
              </button>
              <button class="rounded-md px-2.5 py-1 text-[12px] text-[#8a8a8a] hover:text-white" on:click={() => dismissToast(toast.id)}>Dismiss</button>
            </div>
          </div>
        </div>
      </div>
    {/each}
  </div>
</div>
