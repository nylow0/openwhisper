<script lang="ts">
  // Inline language picker (Settings → Languages). Edits a draft; an Apply bar
  // appears once it differs from the saved settings. Applying restarts the engine.
  // Saved languages are pinned to the top on open and after Apply.
  import { createEventDispatcher, getContext } from 'svelte';
  import { fly } from 'svelte/transition';
  import Flag from './Flag.svelte';
  import Toggle from './Toggle.svelte';
  import { engineStatus, saveSettings, settings } from './app-state';
  import {
    draftFromSettings,
    draftToSettings,
    isDraftDirty,
    pinFirst,
    toggleDraftLanguage,
    type LanguageDraft,
  } from './language-draft';
  import { LANGUAGES, searchLanguages } from './languages';

  const dispatch = createEventDispatcher<{ switchModel: void }>();
  const restartEngine = getContext<() => Promise<void>>('restartEngine');

  let draft: LanguageDraft | null = null;
  let pinned: string[] = [];
  let query = '';

  // Settings load asynchronously; start the draft from them once available.
  $: if (!draft && $settings) reset();
  $: dirty = draft !== null && $settings !== null && isDraftDirty(draft, $settings);
  $: rows = pinFirst(searchLanguages(query), pinned);

  function reset() {
    if (!$settings) return;
    draft = draftFromSettings($settings);
    pinned = draft.auto ? [] : [...draft.languages];
  }

  async function apply() {
    if (!draft || !$settings) return;
    await saveSettings(draftToSettings(draft, $settings));
    reset();
    await restartEngine();
  }
</script>

{#if $settings?.model === 'medium_en_q8'}
  <div class="px-8 pt-6">
    <h2 class="border-b border-ink-700 pb-2 text-[15px] font-bold">Languages you speak</h2>
    <p class="mt-3 max-w-[480px] text-[14px] leading-relaxed text-ink-300">
      The English-only model understands English and nothing else. Switch to the multilingual model to choose from
      {LANGUAGES.length} languages.
    </p>
    <button
      type="button"
      disabled={$engineStatus === 'restarting'}
      class="mt-4 rounded-lg bg-blue-500 px-4 py-1.5 text-[13px] font-semibold text-white hover:bg-blue-400 disabled:opacity-50"
      on:click={() => dispatch('switchModel')}
    >
      Switch to Multilingual
    </button>
  </div>
{:else if draft}
  <div class="flex shrink-0 items-center gap-5 px-8 pb-2 pt-4">
    <input
      type="text"
      bind:value={query}
      placeholder="Search {LANGUAGES.length} languages"
      class="flex-1 border-b border-ink-700 bg-transparent py-2 text-[14px] text-white placeholder:text-ink-500 focus:border-blue-500 focus:outline-none"
    />
    <div class="flex items-center gap-2.5 text-[14px]">
      <Toggle label="Auto-detect" checked={draft.auto} on:change={(event) => draft && (draft = { ...draft, auto: event.detail })} />
      <span aria-hidden="true">Auto-detect</span>
    </div>
  </div>

  <div class="scroll-area min-h-0 flex-1 overflow-y-auto px-8 pb-6">
    {#if rows.length === 0}
      <p class="pt-6 text-[14px] text-ink-400">No language matches “{query}”.</p>
    {/if}
    <div class="grid grid-cols-2 gap-x-8 {draft.auto ? 'opacity-40' : ''}">
      {#each rows as language (language.id)}
        {@const on = !draft.auto && draft.languages.includes(language.id)}
        <button
          type="button"
          role="checkbox"
          aria-checked={on}
          class="flex min-w-0 items-center gap-3 border-b border-ink-850 py-2.5 text-left"
          on:click={() => draft && (draft = toggleDraftLanguage(draft, language.id))}
        >
          <Flag {language} />
          <span class="shrink-0 text-[14px] {on ? 'text-white' : 'text-ink-300'}">{language.name}</span>
          {#if language.nativeName !== language.name}
            <span class="truncate text-[13px] text-ink-500">{language.nativeName}</span>
          {/if}
          <span class="ml-auto text-[13px] {on ? 'text-cyan-400' : 'text-transparent'}" aria-hidden="true">✓</span>
        </button>
      {/each}
    </div>
  </div>

  {#if dirty}
    <footer class="flex shrink-0 items-center gap-5 border-t border-ink-700 px-8 py-3.5" transition:fly={{ y: 20, duration: 180 }}>
      <p class="flex-1 text-[13px] text-ink-200">
        <b class="font-semibold text-white">{draft.auto ? 'Auto-detect.' : `${draft.languages.length} selected.`}</b>
        Applying restarts the speech engine.
      </p>
      <button type="button" class="text-[13px] text-ink-300 hover:text-white" on:click={reset}>Discard</button>
      <button
        type="button"
        disabled={$engineStatus === 'restarting'}
        class="rounded-lg bg-blue-500 px-4 py-1.5 text-[13px] font-semibold text-white hover:bg-blue-400 disabled:opacity-50"
        on:click={apply}
      >
        Apply
      </button>
    </footer>
  {/if}
{/if}
