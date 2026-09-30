<script lang="ts">
  // PROTOTYPE variant C, "Stage": the words are the interface. One big prompt, an editorial
  // serif feed of transcripts, and everything else tucked into a right-hand sheet.
  import { tick } from 'svelte';
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
    toasts,
    toggleDraftLanguage,
    setStartupToggle,
    type LanguageDraft,
    type StartupToggle,
  } from './mock';

  const LAUNCH: StartupToggle = 'launchAtLogin';
  const SHOW: StartupToggle = 'showWindowOnLaunch';

  const LILAC = '#b9a6ff';
  const SERIF = "'Instrument Serif', serif";

  let sheet: 'closed' | 'settings' | 'languages' = 'closed';
  let searching = false;
  let query = '';
  let copiedId: string | null = null;
  let draft: LanguageDraft = { languages: [], auto: false };
  let pinnedLanguages: LanguageDraft['languages'] = [];
  let languageQuery = '';
  let searchInput: HTMLInputElement | null = null;

  $: needle = query.trim().toLowerCase();
  $: groups = groupByDay(needle ? $history.filter((item) => item.text.toLowerCase().includes(needle)) : $history);
  $: selected = languageOptions($settings.spokenLanguages);
  $: chipLabel = $settings.autoDetectLanguage ? 'Any language' : selected.map((language) => language.id.toUpperCase()).join(' · ');

  async function toggleSearch() {
    searching = !searching;
    query = '';
    await tick();
    searchInput?.focus();
  }

  async function copy(id: string, text: string) {
    if (!(await copyText(text))) return;
    copiedId = id;
    setTimeout(() => (copiedId = copiedId === id ? null : copiedId), 1400);
  }

  function openLanguages() {
    draft = draftFromSettings($settings);
    pinnedLanguages = draft.auto ? [] : draft.languages;
    languageQuery = '';
    sheet = 'languages';
  }
</script>

<div class="relative flex h-full flex-col bg-black text-[#ededed]" style="font-family: Geist, sans-serif;">
  <header class="drag flex h-11 shrink-0 items-center pl-6">
    <span class="text-[19px] italic text-white" style="font-family: {SERIF}">OpenWhisper</span>
    <span class="flex-1"></span>
    <div class="no-drag flex items-center gap-1 pr-2">
      <button
        class="mr-1 flex items-center gap-1.5 rounded-full border border-[#222] px-3 py-1 text-[11px] font-medium tracking-wide text-[#a3a3a3] hover:border-[#3a3a3a] hover:text-white"
        on:click={() => (sheet = 'settings')}
      >
        {#if !$settings.autoDetectLanguage}
          {#each selected.slice(0, 3) as language}<Flag {language} size={11} square />{/each}
        {/if}
        {chipLabel}
      </button>
      <button class="rounded-full p-2 text-[#8a8a8a] hover:bg-white/5 hover:text-white" aria-label="Search" on:click={toggleSearch}>
        <svg viewBox="0 0 24 24" class="h-4 w-4" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>
      </button>
      <button class="rounded-full p-2 text-[#8a8a8a] hover:bg-white/5 hover:text-white" aria-label="Settings" on:click={() => (sheet = 'settings')}>
        <svg viewBox="0 0 24 24" class="h-4 w-4" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M4 7h10M18 7h2M4 17h4M12 17h8" /><circle cx="16" cy="7" r="2" /><circle cx="10" cy="17" r="2" /></svg>
      </button>
    </div>
    <WindowControls color="#8a8a8a" />
  </header>

  <!-- Toasts: centred pill with an action -->
  <div class="pointer-events-none absolute left-0 right-0 top-14 z-50 flex flex-col items-center gap-2">
    {#each $toasts as toast (toast.id)}
      <div
        in:fly={{ y: -10, duration: 200 }}
        out:fade={{ duration: 150 }}
        class="pointer-events-auto flex items-center gap-3 rounded-full border border-[#3a1f24] bg-[#0a0a0a] py-2 pl-4 pr-2 text-[13px] shadow-[0_10px_40px_rgba(0,0,0,0.8)]"
      >
        <span class="h-1.5 w-1.5 rounded-full bg-[#ff7a8a]"></span>
        <span class="text-[#d4d4d4]">{toast.message}</span>
        <button
          class="rounded-full bg-white px-3 py-1 text-[12px] font-medium text-black"
          on:click={() => {
            restartEngine();
            dismissToast(toast.id);
          }}
        >
          Restart
        </button>
      </div>
    {/each}
  </div>

  <div class="scroll-area min-h-0 flex-1 overflow-y-auto">
    <div class="mx-auto max-w-[620px] px-6 pb-16">
      <!-- Stage: the one big prompt, or the search field -->
      <section class="pb-10 pt-12">
        {#if searching}
          <input
            bind:this={searchInput}
            bind:value={query}
            on:keydown={(event) => event.key === 'Escape' && toggleSearch()}
            placeholder="Search your words"
            class="w-full border-b border-[#222] bg-transparent pb-3 text-[40px] leading-tight text-white placeholder:text-[#333] focus:border-[#444] focus:outline-none"
            style="font-family: {SERIF}"
          />
          <p class="mt-3 text-[12px] text-[#5c5c5c]">Esc to close</p>
        {:else}
          <p class="text-[11px] font-medium uppercase tracking-[0.18em]" style="color: {$restarting ? '#6b6b6b' : LILAC}">
            {$restarting ? 'Waking the engine…' : 'Ready when you are'}
          </p>
          <h1 class="mt-3 text-[46px] leading-[1.05] text-white" style="font-family: {SERIF}">
            Hold <span class="italic" style="color: {LILAC}">Ctrl + Win</span> and just talk.
          </h1>
          <p class="mt-4 max-w-[440px] text-[14px] leading-relaxed text-[#737373]">
            Let go and your words are typed wherever your cursor is. Nothing leaves this computer.
          </p>
        {/if}
      </section>

      {#if $history.length === 0}
        <p class="border-t border-[#161616] pt-8 text-[22px] italic text-[#3d3d3d]" style="font-family: {SERIF}">
          Your first words will appear here.
        </p>
      {:else if groups.length === 0}
        <p class="text-[22px] italic text-[#4d4d4d]" style="font-family: {SERIF}">Nothing matches “{query}”.</p>
      {:else}
        {#each groups as group}
          <div class="mb-5 flex items-baseline gap-4 border-t border-[#161616] pt-6">
            <h2 class="text-[20px] italic text-[#6b6b6b]" style="font-family: {SERIF}">{group.label}</h2>
          </div>
          <div class="mb-10 space-y-8">
            {#each group.items as item (item.id)}
              <article class="group">
                <div class="mb-1.5 flex items-center gap-3 text-[11px] font-medium uppercase tracking-[0.12em] text-[#4d4d4d]">
                  <span class="tabular-nums">{formatTime(item.createdAt)}</span>
                  <span>{item.language === 'uk' ? 'Українська' : 'English'}</span>
                  <span class="tabular-nums">{item.latencyMs} ms</span>
                  <span class="ml-auto flex gap-3 normal-case tracking-normal opacity-0 transition-opacity group-hover:opacity-100">
                    <button class="text-[12px] text-[#a3a3a3] hover:text-white" on:click={() => copy(item.id, item.text)}>
                      {copiedId === item.id ? 'Copied' : 'Copy'}
                    </button>
                    <button class="text-[12px] text-[#a3a3a3] hover:text-[#ff7a8a]" on:click={() => deleteHistoryItem(item.id)}>Delete</button>
                  </span>
                </div>
                <p class="text-[23px] leading-[1.3] text-[#e8e8e8]" style="font-family: {SERIF}">{item.text}</p>
              </article>
            {/each}
          </div>
        {/each}
      {/if}
    </div>
  </div>

  <!-- Right-hand sheet: settings, then languages stacked on top -->
  {#if sheet !== 'closed'}
    <button class="absolute inset-0 z-30 bg-black/70" aria-label="Close settings" transition:fade={{ duration: 150 }} on:click={() => (sheet = 'closed')}></button>
    <aside
      class="scroll-area absolute bottom-0 right-0 top-0 z-40 flex w-[420px] flex-col overflow-y-auto border-l border-[#1a1a1a] bg-black"
      transition:fly={{ x: 420, duration: 260, opacity: 1 }}
    >
      {#if sheet === 'settings'}
        <div class="flex items-center justify-between px-7 pb-2 pt-7">
          <h2 class="text-[34px] text-white" style="font-family: {SERIF}">Settings</h2>
          <button class="rounded-full p-2 text-[#6b6b6b] hover:bg-white/5 hover:text-white" aria-label="Close" on:click={() => (sheet = 'closed')}>
            <svg viewBox="0 0 24 24" class="h-4 w-4" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M6 6l12 12M18 6 6 18" /></svg>
          </button>
        </div>

        <div class="space-y-8 px-7 pb-8 pt-4">
          <section>
            <p class="mb-3 text-[11px] font-medium uppercase tracking-[0.14em] text-[#5c5c5c]">Voice model</p>
            <div class="grid grid-cols-2 gap-2.5">
              {#each MODELS as model}
                {@const on = $settings.model === model.id}
                <button
                  disabled={$restarting}
                  class="rounded-2xl border p-4 text-left transition-colors disabled:opacity-50 {on ? '' : 'border-[#1f1f1f] hover:border-[#333]'}"
                  style={on ? `border-color: ${LILAC}; background: rgba(185,166,255,0.06)` : ''}
                  on:click={() => !on && patchSettings({ model: model.id })}
                >
                  <p class="text-[22px] leading-none text-white" style="font-family: {SERIF}">{model.name}</p>
                  <p class="mt-2 text-[12px] leading-snug text-[#737373]">{model.detail}</p>
                </button>
              {/each}
            </div>
          </section>

          {#if $settings.model === 'large_v3_turbo_q8'}
            <section>
              <p class="mb-3 text-[11px] font-medium uppercase tracking-[0.14em] text-[#5c5c5c]">Languages you speak</p>
              <div class="flex flex-wrap gap-2">
                {#if $settings.autoDetectLanguage}
                  <span class="rounded-full border border-[#262626] px-3 py-1.5 text-[13px]">Any language</span>
                {:else}
                  {#each selected as language}
                    <span class="flex items-center gap-2 rounded-full border border-[#262626] py-1.5 pl-1.5 pr-3 text-[13px]">
                      <Flag {language} size={18} square />{language.nativeName}
                    </span>
                  {/each}
                {/if}
                <button class="rounded-full border border-dashed border-[#333] px-3 py-1.5 text-[13px] text-[#8a8a8a] hover:border-[#555] hover:text-white" on:click={openLanguages}>
                  Change
                </button>
              </div>
            </section>
          {/if}

          <section>
            <p class="mb-3 text-[11px] font-medium uppercase tracking-[0.14em] text-[#5c5c5c]">Runs on</p>
            <div class="inline-flex rounded-full border border-[#1f1f1f] p-1">
              {#each DEVICES as device}
                <button
                  disabled={$restarting}
                  class="rounded-full px-4 py-1.5 text-[13px] transition-colors {$settings.device === device.id ? 'bg-white text-black' : 'text-[#8a8a8a] hover:text-white'}"
                  on:click={() => $settings.device !== device.id && patchSettings({ device: device.id })}
                >
                  {device.name}
                </button>
              {/each}
            </div>
          </section>

          <section class="space-y-4">
            <p class="text-[11px] font-medium uppercase tracking-[0.14em] text-[#5c5c5c]">When Windows starts</p>
            {#each [{ label: 'Launch OpenWhisper', key: LAUNCH }, { label: 'Show this window', key: SHOW }] as row}
              {@const on = $settings[row.key]}
              <button class="flex w-full items-center justify-between text-[14px]" on:click={() => setStartupToggle(row.key, !on)}>
                {row.label}
                <span class="relative h-6 w-10 rounded-full transition-colors" style="background: {on ? LILAC : '#1f1f1f'}">
                  <span class="absolute top-1 h-4 w-4 rounded-full transition-all {on ? 'left-5 bg-black' : 'left-1 bg-[#6b6b6b]'}"></span>
                </span>
              </button>
            {/each}
          </section>

          <section class="flex gap-2">
            <button
              disabled={$restarting}
              class="rounded-full border border-[#262626] px-4 py-2 text-[13px] text-[#d4d4d4] hover:border-[#444] disabled:animate-breathe"
              on:click={restartEngine}
            >
              {$restarting ? 'Restarting…' : 'Restart engine'}
            </button>
            <button class="rounded-full border border-[#262626] px-4 py-2 text-[13px] text-[#8a8a8a] hover:border-[#5c2a33] hover:text-[#ff7a8a]" on:click={clearHistory}>
              Clear history
            </button>
          </section>

          <p class="text-[16px] italic text-[#4d4d4d]" style="font-family: {SERIF}">OpenWhisper 0.1.0. Offline, always.</p>
        </div>
      {:else}
        <!-- Language grid -->
        <div class="flex items-center gap-2 px-7 pb-2 pt-7">
          <button class="-ml-2 rounded-full p-2 text-[#6b6b6b] hover:bg-white/5 hover:text-white" aria-label="Back" on:click={() => (sheet = 'settings')}>
            <svg viewBox="0 0 24 24" class="h-4 w-4" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M15 6l-6 6 6 6" /></svg>
          </button>
          <h2 class="text-[34px] text-white" style="font-family: {SERIF}">Languages</h2>
        </div>
        <div class="px-7">
          <input
            bind:value={languageQuery}
            placeholder="Find a language"
            class="w-full rounded-full border border-[#1f1f1f] bg-transparent px-4 py-2 text-[13px] text-white placeholder:text-[#4d4d4d] focus:border-[#3a3a3a] focus:outline-none"
          />
          <button class="mt-3 flex w-full items-center justify-between rounded-2xl border border-[#1a1a1a] px-4 py-3 text-left" on:click={() => (draft = { ...draft, auto: !draft.auto })}>
            <span>
              <span class="block text-[14px]">Detect automatically</span>
              <span class="block text-[12px] text-[#5c5c5c]">Listens for all 99 languages. A little slower.</span>
            </span>
            <span class="relative h-6 w-10 shrink-0 rounded-full transition-colors" style="background: {draft.auto ? LILAC : '#1f1f1f'}">
              <span class="absolute top-1 h-4 w-4 rounded-full transition-all {draft.auto ? 'left-5 bg-black' : 'left-1 bg-[#6b6b6b]'}"></span>
            </span>
          </button>
        </div>
        <div class="grid flex-1 grid-cols-3 content-start gap-2 px-7 py-4 {draft.auto ? 'opacity-40' : ''}">
          {#each filterLanguages(languageQuery, pinnedLanguages) as language (language.id)}
            {@const on = !draft.auto && draft.languages.includes(language.id)}
            <button
              class="flex flex-col items-start gap-2 rounded-2xl border p-3 text-left transition-colors {on ? '' : 'border-[#161616] hover:border-[#2a2a2a]'}"
              style={on ? `border-color: ${LILAC}; background: rgba(185,166,255,0.06)` : ''}
              on:click={() => (draft = toggleDraftLanguage(draft, language.id))}
            >
              <Flag {language} size={22} square />
              <span class="w-full">
                <span class="block truncate text-[13px] {on ? 'text-white' : 'text-[#bdbdbd]'}">{language.nativeName}</span>
                <span class="block truncate text-[11px] text-[#5c5c5c]">{language.name}</span>
              </span>
            </button>
          {/each}
        </div>
        <div class="sticky bottom-0 flex justify-end gap-2 border-t border-[#161616] bg-black px-7 py-4">
          <button class="rounded-full px-4 py-2 text-[13px] text-[#8a8a8a] hover:text-white" on:click={() => (sheet = 'settings')}>Cancel</button>
          <button
            class="rounded-full px-5 py-2 text-[13px] font-semibold text-black"
            style="background: {LILAC}"
            on:click={() => {
              saveDraft(draft);
              sheet = 'settings';
            }}
          >
            Done
          </button>
        </div>
      {/if}
    </aside>
  {/if}
</div>
