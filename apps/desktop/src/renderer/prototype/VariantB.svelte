<script lang="ts">
  // PROTOTYPE variant B, "Console": keyboard-first and dense. Monospace chrome, a transcript
  // table, settings as a config file, a command-palette language picker, and a status line
  // that doubles as the error surface.
  import { onMount, tick } from 'svelte';
  import { fade } from 'svelte/transition';
  import WindowControls from './WindowControls.svelte';
  import Flag from './Flag.svelte';
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

  const AMBER = '#ffb224';

  let view: 'history' | 'settings' = 'history';
  let query = '';
  let selectedIndex = 0;
  let flash = '';
  let pickerOpen = false;
  let draft: LanguageDraft = { languages: [], auto: false };
  let pinnedLanguages: LanguageDraft['languages'] = [];
  let pickerQuery = '';
  let pickerIndex = 0;
  let pickerInput: HTMLInputElement | null = null;
  let searchInput: HTMLInputElement | null = null;

  $: needle = query.trim().toLowerCase();
  $: rows = needle ? $history.filter((item) => item.text.toLowerCase().includes(needle)) : $history;
  $: if (selectedIndex >= rows.length) selectedIndex = Math.max(0, rows.length - 1);
  $: pickerRows = filterLanguages(pickerQuery, pinnedLanguages);
  $: if (pickerIndex > pickerRows.length) pickerIndex = 0;
  $: toast = $toasts[0];
  $: modelKey = $settings.model === 'large_v3_turbo_q8' ? 'large-v3-turbo' : 'medium-en';
  $: languagesKey = $settings.autoDetectLanguage ? 'auto' : $settings.spokenLanguages.join(',');

  function say(message: string) {
    flash = message;
    setTimeout(() => (flash = flash === message ? '' : flash), 1400);
  }

  async function copySelected() {
    const row = rows[selectedIndex];
    if (row && (await copyText(row.text))) say(`copied ${wordCount(row.text)} words`);
  }

  function deleteSelected() {
    const row = rows[selectedIndex];
    if (!row) return;
    deleteHistoryItem(row.id);
    say('deleted 1 transcript');
  }

  async function openPicker() {
    draft = draftFromSettings($settings);
    pinnedLanguages = draft.auto ? [] : draft.languages;
    pickerQuery = '';
    pickerIndex = 0;
    pickerOpen = true;
    await tick();
    pickerInput?.focus();
  }

  function togglePickerRow(index: number) {
    if (index === 0) draft = { ...draft, auto: !draft.auto };
    else draft = toggleDraftLanguage(draft, pickerRows[index - 1].id);
  }

  function onPickerKey(event: KeyboardEvent) {
    const total = pickerRows.length + 1;
    if (event.key === 'ArrowDown') pickerIndex = (pickerIndex + 1) % total;
    else if (event.key === 'ArrowUp') pickerIndex = (pickerIndex - 1 + total) % total;
    else if (event.key === ' ' && pickerQuery === '') togglePickerRow(pickerIndex);
    else if (event.key === 'Tab') togglePickerRow(pickerIndex);
    else if (event.key === 'Enter') {
      saveDraft(draft);
      pickerOpen = false;
    } else if (event.key === 'Escape') pickerOpen = false;
    else return;
    event.preventDefault();
  }

  onMount(() => {
    function onKey(event: KeyboardEvent) {
      if (pickerOpen) return;
      const target = event.target instanceof Element ? event.target : null;
      if (target?.closest('input')) {
        if (event.key === 'Escape' && target instanceof HTMLInputElement) target.blur();
        return;
      }
      if (event.key === '1') view = 'history';
      else if (event.key === '2') view = 'settings';
      else if (event.key === '/') {
        view = 'history';
        tick().then(() => searchInput?.focus());
      } else if (view !== 'history') return;
      else if (event.key === 'ArrowDown' || event.key === 'j') selectedIndex = Math.min(rows.length - 1, selectedIndex + 1);
      else if (event.key === 'ArrowUp' || event.key === 'k') selectedIndex = Math.max(0, selectedIndex - 1);
      else if (event.key === 'c') copySelected();
      else if (event.key === 'd' || event.key === 'Delete') deleteSelected();
      else return;
      event.preventDefault();
    }
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });
</script>

<div class="relative flex h-full flex-col bg-black text-[#e6e6e6]" style="font-family: 'Geist Mono', monospace;">
  <!-- Title bar with live engine status -->
  <header class="drag flex h-11 shrink-0 items-center gap-4 border-b border-[#141414] pl-5 text-[12px]">
    <span class="text-white">openwhisper</span>
    <span class="flex items-center gap-2 text-[#6b6b6b]">
      <span class="h-1.5 w-1.5 rounded-full {$restarting ? 'animate-breathe' : ''}" style="background: {$restarting ? '#6b6b6b' : AMBER}"></span>
      {$restarting ? 'restarting' : 'ready'} · {modelKey} · {$settings.device} · {languagesKey}
    </span>
    <span class="flex-1"></span>
    <WindowControls color="#8a8a8a" />
  </header>

  <!-- Tabs + search/command line -->
  <div class="flex h-11 shrink-0 items-center gap-1 border-b border-[#141414] px-3 text-[12px]">
    {#each [{ id: 'history', key: '1' }, { id: 'settings', key: '2' }] as tab}
      <button
        class="relative flex h-full items-center gap-2 px-2.5 {view === tab.id ? 'text-white' : 'text-[#5c5c5c] hover:text-[#a3a3a3]'}"
        on:click={() => (view = tab.id === 'history' ? 'history' : 'settings')}
      >
        <span class="text-[#3d3d3d]">{tab.key}</span>{tab.id}
        {#if view === tab.id}<span class="absolute inset-x-2 -bottom-px h-px" style="background: {AMBER}"></span>{/if}
      </button>
    {/each}
    <label class="ml-auto flex w-[340px] items-center gap-2 rounded border border-[#1f1f1f] px-2.5 py-1.5 focus-within:border-[#3d3d3d]">
      <span style="color: {AMBER}">/</span>
      <input
        bind:this={searchInput}
        bind:value={query}
        on:focus={() => (view = 'history')}
        placeholder="search transcripts"
        class="flex-1 bg-transparent text-[12px] text-white placeholder:text-[#4d4d4d] focus:outline-none"
      />
      <span class="text-[10px] text-[#3d3d3d]">esc</span>
    </label>
  </div>

  <div class="scroll-area min-h-0 flex-1 overflow-y-auto">
    {#if view === 'history'}
      {#if $history.length === 0}
        <div class="flex h-full flex-col items-center justify-center gap-2 text-[12px] text-[#5c5c5c]" in:fade={{ duration: 120 }}>
          <p class="text-white">no transcripts yet</p>
          <p>hold <span style="color: {AMBER}">ctrl+win</span> · speak · release</p>
          <p>text is typed at your cursor and logged here</p>
        </div>
      {:else}
        <div class="sticky top-0 z-10 grid grid-cols-[76px_44px_56px_1fr] gap-3 border-b border-[#141414] bg-black px-5 py-2 text-[10px] uppercase tracking-wider text-[#4d4d4d]">
          <span>time</span><span>lang</span><span class="text-right">ms</span><span>text</span>
        </div>
        {#each rows as item, index (item.id)}
          {@const active = index === selectedIndex}
          <button
            class="relative grid w-full grid-cols-[76px_44px_56px_1fr] gap-3 border-b border-[#0d0d0d] px-5 py-2 text-left text-[12px] {active
              ? 'bg-[#0b0b0b]'
              : 'hover:bg-[#070707]'}"
            on:click={() => (selectedIndex = index)}
          >
            {#if active}<span class="absolute inset-y-0 left-0 w-[2px]" style="background: {AMBER}"></span>{/if}
            <span class="tabular-nums text-[#6b6b6b]">{formatTime(item.createdAt)}</span>
            <span class={item.language === 'uk' ? 'text-[#a3a3a3]' : 'text-[#6b6b6b]'}>{item.language}</span>
            <span class="text-right tabular-nums text-[#6b6b6b]">{item.latencyMs}</span>
            <span class="min-w-0" style="font-family: Geist, sans-serif;">
              <span class="block text-[13px] {active ? 'whitespace-normal text-white' : 'truncate text-[#bdbdbd]'}">{item.text}</span>
              {#if active}
                <span class="mt-1.5 flex gap-4 text-[11px] text-[#5c5c5c]" style="font-family: 'Geist Mono', monospace;">
                  <span><span style="color: {AMBER}">c</span> copy</span>
                  <span><span style="color: {AMBER}">d</span> delete</span>
                  <span>{wordCount(item.text)} words</span>
                </span>
              {/if}
            </span>
          </button>
        {/each}
        {#if rows.length === 0}
          <p class="px-5 py-6 text-[12px] text-[#5c5c5c]">no match for "{query}"</p>
        {/if}
      {/if}
    {:else}
      <!-- Settings as an editable config file -->
      <div class="px-5 py-4 text-[12px] leading-[2.1]" in:fade={{ duration: 120 }}>
        <p class="text-[#3d3d3d]"># speech model — changes restart the engine</p>
        <div class="grid grid-cols-[210px_1fr]">
          <span class="text-[#8a8a8a]">model</span>
          <span class="flex gap-3">
            {#each MODELS as model}
              <button
                disabled={$restarting}
                on:click={() => $settings.model !== model.id && patchSettings({ model: model.id })}
                class={$settings.model === model.id ? 'text-white' : 'text-[#4d4d4d] hover:text-[#a3a3a3]'}
              >
                {#if $settings.model === model.id}<span style="color: {AMBER}">[</span>{/if}{model.id === 'large_v3_turbo_q8' ? 'multilingual' : 'english-only'}{#if $settings.model === model.id}<span style="color: {AMBER}">]</span>{/if}
              </button>
            {/each}
            <span class="text-[#3d3d3d]"># {MODELS.find((model) => model.id === $settings.model)?.size}</span>
          </span>

          <span class="text-[#8a8a8a]">device</span>
          <span class="flex gap-3">
            {#each DEVICES as device}
              <button
                disabled={$restarting}
                on:click={() => $settings.device !== device.id && patchSettings({ device: device.id })}
                class={$settings.device === device.id ? 'text-white' : 'text-[#4d4d4d] hover:text-[#a3a3a3]'}
              >
                {#if $settings.device === device.id}<span style="color: {AMBER}">[</span>{/if}{device.id}{#if $settings.device === device.id}<span style="color: {AMBER}">]</span>{/if}
              </button>
            {/each}
            <span class="text-[#3d3d3d]"># auto prefers gpu</span>
          </span>

          {#if $settings.model === 'large_v3_turbo_q8'}
            <span class="text-[#8a8a8a]">languages</span>
            <span class="flex gap-3">
              <span class="text-white">{$settings.autoDetectLanguage ? 'auto-detect' : $settings.spokenLanguages.join(', ')}</span>
              <button class="hover:underline" style="color: {AMBER}" on:click={openPicker}>edit →</button>
            </span>
          {/if}
        </div>

        <p class="mt-5 text-[#3d3d3d]"># startup</p>
        <div class="grid grid-cols-[210px_1fr]">
          {#each [{ label: 'launch_at_login', key: LAUNCH }, { label: 'open_window_on_launch', key: SHOW }] as row}
            {@const on = $settings[row.key]}
            <span class="text-[#8a8a8a]">{row.label}</span>
            <span class="flex gap-3">
              {#each [true, false] as value}
                <button on:click={() => setStartupToggle(row.key, value)} class={on === value ? 'text-white' : 'text-[#4d4d4d] hover:text-[#a3a3a3]'}>
                  {#if on === value}<span style="color: {AMBER}">[</span>{/if}{value ? 'on' : 'off'}{#if on === value}<span style="color: {AMBER}">]</span>{/if}
                </button>
              {/each}
            </span>
          {/each}
        </div>

        <p class="mt-5 text-[#3d3d3d]"># dictation</p>
        <div class="grid grid-cols-[210px_1fr]">
          <span class="text-[#8a8a8a]">shortcut</span>
          <span class="text-white">ctrl+win <span class="text-[#3d3d3d]"># hold to record, release to type</span></span>
          <span class="text-[#8a8a8a]">engine</span>
          <span class="flex gap-3">
            <span class={$restarting ? 'animate-breathe text-[#6b6b6b]' : 'text-white'}>{$restarting ? 'restarting…' : 'running'}</span>
            {#if !$restarting}<button class="hover:underline" style="color: {AMBER}" on:click={restartEngine}>restart</button>{/if}
          </span>
          <span class="text-[#8a8a8a]">history</span>
          <span class="flex gap-3">
            <span class="text-white">{$stats.count} items</span>
            {#if $stats.count > 0}<button class="text-[#6b6b6b] hover:text-[#ff6b6b]" on:click={clearHistory}>clear</button>{/if}
          </span>
        </div>

        <p class="mt-5 text-[#3d3d3d]"># openwhisper 0.1.0 · offline · audio never leaves this machine</p>
      </div>
    {/if}
  </div>

  <!-- Status line: stats, key hints, and errors -->
  <footer class="flex h-8 shrink-0 items-center gap-4 border-t border-[#141414] px-5 text-[11px]">
    {#if toast}
      <span class="flex min-w-0 flex-1 items-center gap-2 text-[#ff6b6b]" in:fade={{ duration: 100 }}>
        <span class="rounded-sm bg-[#ff6b6b] px-1 text-black">ERR</span>
        <span class="truncate">{toast.code.toLowerCase()} · {toast.message}</span>
        <button class="ml-auto text-[#6b6b6b] hover:text-white" on:click={() => dismissToast(toast.id)}>[x]</button>
      </span>
    {:else}
      <span class="text-[#5c5c5c]">
        {flash || `${$stats.count} transcripts · ${$stats.words} words · avg ${$stats.avgLatencyMs ?? '—'}ms`}
      </span>
      <span class="ml-auto flex gap-4 text-[#3d3d3d]">
        {#if view === 'history'}
          <span><span class="text-[#8a8a8a]">↑↓</span> select</span>
          <span><span class="text-[#8a8a8a]">c</span> copy</span>
          <span><span class="text-[#8a8a8a]">d</span> delete</span>
        {/if}
        <span><span class="text-[#8a8a8a]">/</span> search</span>
        <span><span class="text-[#8a8a8a]">1 2</span> tabs</span>
      </span>
    {/if}
  </footer>

  <!-- Command-palette language picker -->
  {#if pickerOpen}
    <div class="absolute inset-0 z-40 flex items-start justify-center bg-black/70 pt-20" transition:fade={{ duration: 100 }}>
      <button class="absolute inset-0 cursor-default" aria-label="Close" on:click={() => (pickerOpen = false)}></button>
      <div class="relative w-[540px] overflow-hidden rounded-md border border-[#262626] bg-black shadow-[0_20px_80px_rgba(0,0,0,0.9)]">
        <label class="flex items-center gap-2.5 border-b border-[#1a1a1a] px-4 py-3 text-[13px]">
          <span style="color: {AMBER}">&gt;</span>
          <input
            bind:this={pickerInput}
            bind:value={pickerQuery}
            on:keydown={onPickerKey}
            on:input={() => (pickerIndex = 1)}
            placeholder="languages · type to filter"
            class="flex-1 bg-transparent text-white placeholder:text-[#4d4d4d] focus:outline-none"
          />
          <span class="text-[11px] text-[#4d4d4d]">{draft.auto ? 'auto' : `${draft.languages.length} on`}</span>
        </label>
        <div class="scroll-area max-h-[340px] overflow-y-auto py-1 text-[12px]">
          {#each [null, ...pickerRows] as language, index}
            <button
              class="flex w-full items-center gap-3 px-4 py-1.5 text-left {pickerIndex === index ? 'bg-[#111]' : ''}"
              on:mouseenter={() => (pickerIndex = index)}
              on:click={() => togglePickerRow(index)}
            >
              {#if language === null}
                <span style="color: {draft.auto ? AMBER : '#4d4d4d'}">[{draft.auto ? 'x' : ' '}]</span>
                <span class="text-white">auto-detect</span>
                <span class="text-[#4d4d4d]">listen for every language</span>
              {:else}
                {@const on = !draft.auto && draft.languages.includes(language.id)}
                <span style="color: {on ? AMBER : '#4d4d4d'}">[{on ? 'x' : ' '}]</span>
                <span class="w-9 text-[#6b6b6b]">{language.id}</span>
                <Flag {language} size={10} />
                <span class={on ? 'text-white' : 'text-[#a3a3a3]'} style="font-family: Geist, sans-serif;">{language.name}</span>
                {#if language.nativeName !== language.name}
                  <span class="truncate text-[#4d4d4d]" style="font-family: Geist, sans-serif;">{language.nativeName}</span>
                {/if}
              {/if}
            </button>
          {/each}
        </div>
        <div class="flex gap-4 border-t border-[#1a1a1a] px-4 py-2 text-[11px] text-[#4d4d4d]">
          <span><span class="text-[#8a8a8a]">↑↓</span> move</span>
          <span><span class="text-[#8a8a8a]">tab</span> toggle</span>
          <span><span class="text-[#8a8a8a]">enter</span> save</span>
          <span><span class="text-[#8a8a8a]">esc</span> cancel</span>
        </div>
      </div>
    </div>
  {/if}
</div>
