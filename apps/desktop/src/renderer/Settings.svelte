<script lang="ts">
  import { getContext, onMount, tick } from 'svelte';
  import {
    SUPPORTED_ASR_LANGUAGES,
    type AppSettings,
    type AsrDevice,
    type AsrLanguageCode,
    type AsrModel,
  } from '../shared/types';
  import 'flag-icons/css/flag-icons.min.css';
  import bashkortostanFlag from './flags/bashkortostan.svg';
  import tatarstanFlag from './flags/tatarstan.svg';

  const api = window.api;
  const pushToast = getContext<(message: string) => void>('pushToast');
  const setExpectedEngineRestart = getContext<(expected: boolean) => void>(
    'setExpectedEngineRestart'
  );

  const MODELS: Array<{ id: AsrModel; name: string; detail: string }> = [
    { id: 'large_v3_turbo_q8', name: 'Multilingual', detail: 'Recommended for accuracy and GPU speed' },
    { id: 'medium_en_q8', name: 'English only', detail: 'Smaller fallback for English dictation' },
  ];
  const DEVICES: Array<{ id: AsrDevice; name: string }> = [
    { id: 'auto', name: 'Auto' },
    { id: 'cpu', name: 'CPU' },
    { id: 'gpu', name: 'GPU' },
  ];

  type Language = (typeof SUPPORTED_ASR_LANGUAGES)[number];
  type LanguageOption = Language & {
    flag: string;
    flagImage?: string;
    nativeName: string;
    searchText: string;
  };

  // flag-icons code overrides for languages whose flag is a subdivision, not a country.
  const FLAG_CODE_OVERRIDES: Partial<Record<AsrLanguageCode, string>> = {
    cy: 'gb-wls', // Welsh → Wales
  };
  // Custom flag images for regions with no ISO country code in flag-icons.
  const FLAG_IMAGE_OVERRIDES: Partial<Record<AsrLanguageCode, string>> = {
    ba: bashkortostanFlag, // Bashkir → Bashkortostan
    tt: tatarstanFlag, // Tatar → Tatarstan
  };

  const LANGUAGE_FLAGS: Record<AsrLanguageCode, string> = {
    en: '🇬🇧',
    zh: '🇨🇳',
    de: '🇩🇪',
    es: '🇪🇸',
    ru: '🇷🇺',
    ko: '🇰🇷',
    fr: '🇫🇷',
    ja: '🇯🇵',
    pt: '🇵🇹',
    tr: '🇹🇷',
    pl: '🇵🇱',
    ca: '🇪🇸',
    nl: '🇳🇱',
    ar: '🇸🇦',
    sv: '🇸🇪',
    it: '🇮🇹',
    id: '🇮🇩',
    hi: '🇮🇳',
    fi: '🇫🇮',
    vi: '🇻🇳',
    he: '🇮🇱',
    uk: '🇺🇦',
    el: '🇬🇷',
    ms: '🇲🇾',
    cs: '🇨🇿',
    ro: '🇷🇴',
    da: '🇩🇰',
    hu: '🇭🇺',
    ta: '🇮🇳',
    no: '🇳🇴',
    th: '🇹🇭',
    ur: '🇵🇰',
    hr: '🇭🇷',
    bg: '🇧🇬',
    lt: '🇱🇹',
    la: '🇻🇦',
    mi: '🇳🇿',
    ml: '🇮🇳',
    cy: '🇬🇧',
    sk: '🇸🇰',
    te: '🇮🇳',
    fa: '🇮🇷',
    lv: '🇱🇻',
    bn: '🇮🇳',
    sr: '🇷🇸',
    az: '🇦🇿',
    sl: '🇸🇮',
    kn: '🇮🇳',
    et: '🇪🇪',
    mk: '🇲🇰',
    br: '🇫🇷',
    eu: '🇪🇸',
    is: '🇮🇸',
    hy: '🇦🇲',
    ne: '🇳🇵',
    mn: '🇲🇳',
    bs: '🇧🇦',
    kk: '🇰🇿',
    sq: '🇦🇱',
    sw: '🇹🇿',
    gl: '🇪🇸',
    mr: '🇮🇳',
    pa: '🇮🇳',
    si: '🇱🇰',
    km: '🇰🇭',
    sn: '🇿🇼',
    yo: '🇳🇬',
    so: '🇸🇴',
    af: '🇿🇦',
    oc: '🇫🇷',
    ka: '🇬🇪',
    be: '🇧🇾',
    tg: '🇹🇯',
    sd: '🇵🇰',
    gu: '🇮🇳',
    am: '🇪🇹',
    yi: '🇮🇱',
    lo: '🇱🇦',
    uz: '🇺🇿',
    fo: '🇫🇴',
    ht: '🇭🇹',
    ps: '🇦🇫',
    tk: '🇹🇲',
    nn: '🇳🇴',
    mt: '🇲🇹',
    sa: '🇮🇳',
    lb: '🇱🇺',
    my: '🇲🇲',
    bo: '🇨🇳',
    tl: '🇵🇭',
    mg: '🇲🇬',
    as: '🇮🇳',
    tt: '🇷🇺',
    haw: '🇺🇸',
    ln: '🇨🇩',
    ha: '🇳🇬',
    ba: '🇷🇺',
    jw: '🇮🇩',
    su: '🇮🇩',
    yue: '🇭🇰',
  };

  const LANGUAGES: LanguageOption[] = [...SUPPORTED_ASR_LANGUAGES]
    .sort((a, b) => a.name.localeCompare(b.name))
    .map((language) => {
      const nativeName = nativeLanguageName(language.id, language.name);
      return {
        ...language,
        flag: FLAG_CODE_OVERRIDES[language.id] ?? flagEmojiToCode(LANGUAGE_FLAGS[language.id]),
        flagImage: FLAG_IMAGE_OVERRIDES[language.id],
        nativeName,
        searchText: `${language.name} ${nativeName} ${language.id}`.toLowerCase(),
      };
    });
  const LANGUAGE_BY_ID = new Map<AsrLanguageCode, LanguageOption>(
    LANGUAGES.map((language) => [language.id, language])
  );

  let settings: AppSettings | null = null;
  let restarting = false;
  let languagePickerOpen = false;
  let languageSearch = '';
  let selectedLanguages: AsrLanguageCode[] = ['en'];
  let draftLanguages: AsrLanguageCode[] = ['en'];
  let draftAutoDetect = false;
  let languageSearchInput: HTMLInputElement | null = null;

  $: selectedLanguages =
    settings?.model === 'medium_en_q8' ? ['en'] : settings?.spokenLanguages ?? ['en'];
  $: selectedLanguageOptions = selectedLanguages
    .map((language) => LANGUAGE_BY_ID.get(language))
    .filter(isLanguageOption);
  $: normalizedLanguageSearch = languageSearch.trim().toLowerCase();
  $: filteredLanguages = normalizedLanguageSearch
    ? LANGUAGES.filter((language) => language.searchText.includes(normalizedLanguageSearch))
    : LANGUAGES;
  $: draftLanguageOptions = draftLanguages
    .map((language) => LANGUAGE_BY_ID.get(language))
    .filter(isLanguageOption);

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
    setExpectedEngineRestart(true);
    const result = await api.restartEngine();
    restarting = false;
    if (result.ok) {
      setTimeout(() => setExpectedEngineRestart(false), 1_000);
      return;
    }
    pushToast(
      result.ok
        ? `${label} — speech engine restarted.`
        : `Engine restart failed: ${result.error ?? 'unknown error'}`
    );
    setTimeout(() => setExpectedEngineRestart(false), 1_000);
  }

  async function chooseModel(model: AsrModel) {
    if (!settings || restarting || settings.model === model) return;
    await patch({ model });
    await applyEngineChange('Model updated');
    if (model === 'large_v3_turbo_q8') {
      await openLanguagePicker();
    }
  }

  async function chooseDevice(device: AsrDevice) {
    if (!settings || restarting || settings.device === device) return;
    await patch({ device });
    await applyEngineChange('Compute device updated');
  }

  function isLanguageOption(value: LanguageOption | undefined): value is LanguageOption {
    return value !== undefined;
  }

  function nativeLanguageName(language: AsrLanguageCode, fallback: string): string {
    try {
      const name = new Intl.DisplayNames([language], { type: 'language' }).of(language);
      if (!name) return fallback;
      return name.charAt(0).toLocaleUpperCase(language) + name.slice(1);
    } catch {
      return fallback;
    }
  }

  /** Convert a flag emoji (two regional-indicator symbols) to its ISO 3166-1 country code. */
  function flagEmojiToCode(flag: string): string {
    const chars = [...flag];
    if (chars.length !== 2) return '';
    const base = 0x1f1e6;
    return chars
      .map((char) => {
        const codePoint = char.codePointAt(0);
        return codePoint ? String.fromCharCode(codePoint - base + 0x61) : '';
      })
      .join('');
  }

  function sameLanguages(left: AsrLanguageCode[], right: AsrLanguageCode[]): boolean {
    return left.length === right.length && left.every((language, index) => language === right[index]);
  }

  async function openLanguagePicker() {
    if (!settings || restarting) return;
    draftLanguages = [...selectedLanguages];
    draftAutoDetect =
      settings.model === 'large_v3_turbo_q8' ? settings.autoDetectLanguage : false;
    languageSearch = '';
    languagePickerOpen = true;
    await tick();
    languageSearchInput?.focus();
  }

  function closeLanguagePicker() {
    languagePickerOpen = false;
  }

  function toggleDraftLanguage(language: AsrLanguageCode) {
    if (draftAutoDetect) {
      // Picking a language exits auto-detect.
      draftAutoDetect = false;
      draftLanguages = [language];
      return;
    }
    const isSelected = draftLanguages.includes(language);
    if (isSelected && draftLanguages.length === 1) {
      // Unselecting the last language falls back to auto-detect.
      draftAutoDetect = true;
      return;
    }

    draftLanguages = isSelected
      ? draftLanguages.filter((selectedLanguage) => selectedLanguage !== language)
      : [...draftLanguages, language];
  }

  function removeDraftLanguage(language: AsrLanguageCode) {
    if (draftAutoDetect) return;
    if (draftLanguages.length === 1) {
      // Removing the last language falls back to auto-detect.
      draftAutoDetect = true;
      return;
    }
    draftLanguages = draftLanguages.filter((selectedLanguage) => selectedLanguage !== language);
  }

  function toggleDraftAutoDetect() {
    draftAutoDetect = !draftAutoDetect;
  }

  async function saveLanguagePicker() {
    if (!settings || restarting) return;

    const spokenLanguages = draftLanguages.length > 0 ? draftLanguages : selectedLanguages;
    const partial: Partial<AppSettings> = {
      model: 'large_v3_turbo_q8',
      spokenLanguages,
      autoDetectLanguage: draftAutoDetect,
    };
    const didChange =
      settings.model !== partial.model ||
      settings.autoDetectLanguage !== draftAutoDetect ||
      !sameLanguages(selectedLanguages, spokenLanguages);

    languagePickerOpen = false;
    if (!didChange) return;

    await patch(partial);
    await applyEngineChange('Languages updated');
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

        {#if settings?.model === 'large_v3_turbo_q8'}
          <div class="mt-4 border-t border-zinc-800 pt-4">
            <div class="flex items-center justify-between gap-3">
              <div class="min-w-0">
                <p class="text-[13px] font-medium text-zinc-200">Languages</p>
                <div class="mt-1.5 flex flex-wrap gap-1.5">
                  {#if settings.autoDetectLanguage}
                    <span class="inline-flex items-center rounded-md border border-indigo-500/40 bg-indigo-500/10 px-2 py-1 text-[11px] font-semibold text-indigo-200">
                      Auto-detect
                    </span>
                  {:else}
                    {#each selectedLanguageOptions as language}
                      <span class="inline-flex items-center gap-1.5 rounded-md border border-indigo-500/40 bg-indigo-500/10 px-2 py-1 text-[11px] font-semibold text-indigo-100">
                        {#if language.flagImage}
                          <img
                            src={language.flagImage}
                            alt=""
                            class="h-3 w-4 shrink-0 rounded-[2px] object-cover ring-1 ring-inset ring-white/15"
                          />
                        {:else if language.flag}
                          <span
                            class="fi fi-{language.flag} shrink-0 rounded-[2px] text-[12px] ring-1 ring-inset ring-white/15"
                            aria-hidden="true"
                          ></span>
                        {/if}
                        {language.name}
                      </span>
                    {/each}
                  {/if}
                </div>
              </div>
              <button
                type="button"
                on:click={openLanguagePicker}
                disabled={restarting}
                class="inline-flex min-w-24 shrink-0 items-center justify-center rounded-lg border border-zinc-700 bg-zinc-800 px-4 py-2 text-[12px] font-semibold text-zinc-100 transition-colors hover:bg-zinc-700 disabled:opacity-60"
              >
                Change
              </button>
            </div>
          </div>
        {/if}

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
            Changing the model, device, or language restarts the speech engine.
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

{#if languagePickerOpen}
  <div class="fixed inset-0 z-40 flex items-center justify-center bg-black/60 px-5 py-5">
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Choose languages"
      class="flex max-h-[calc(100vh-280px)] w-full max-w-4xl flex-col overflow-hidden rounded-2xl border border-zinc-700 bg-zinc-950 shadow-2xl shadow-black/60"
    >
      <div class="flex shrink-0 items-start justify-between gap-4 border-b border-zinc-800 px-6 py-5">
        <div>
          <h2 class="text-lg font-semibold text-zinc-100">Languages</h2>
          <p class="mt-1 text-[13px] text-zinc-500">Select the languages you speak most often.</p>
          {#if draftAutoDetect}
            <p class="mt-1 text-[12px] text-zinc-600">
              Auto-detect is on — pick a language to choose it manually.
            </p>
          {:else}
            <p class="mt-1 text-[12px] text-zinc-600">
              Tip: clear every language to switch back to auto-detect.
            </p>
          {/if}
        </div>
        <div class="flex items-center gap-4">
          <button
            type="button"
            role="switch"
            aria-label="Auto-detect language"
            aria-checked={draftAutoDetect}
            on:click={toggleDraftAutoDetect}
            class="flex items-center gap-2 text-[12px] font-semibold text-zinc-200"
          >
            Auto-detect
            <span
              class="relative h-7 w-12 rounded-full transition-colors {draftAutoDetect
                ? 'bg-indigo-500'
                : 'bg-zinc-700'}"
            >
              <span
                class="absolute top-1 h-5 w-5 rounded-full bg-white transition-all {draftAutoDetect
                  ? 'left-6'
                  : 'left-1'}"
              ></span>
            </span>
          </button>
          <button
            type="button"
            on:click={closeLanguagePicker}
            class="rounded-lg p-2 text-zinc-500 transition-colors hover:bg-zinc-900 hover:text-zinc-200"
            aria-label="Close language picker"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" class="h-4 w-4" aria-hidden="true">
              <path d="M18 6 6 18M6 6l12 12" />
            </svg>
          </button>
        </div>
      </div>

      <div class="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_230px] overflow-hidden">
        <div class="flex min-h-0 flex-col border-r border-zinc-800">
          <label class="flex h-14 shrink-0 items-center gap-3 border-b border-zinc-800 px-5 text-zinc-500">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" class="h-4 w-4" aria-hidden="true">
              <circle cx="11" cy="11" r="7" />
              <path d="m20 20-3.5-3.5" />
            </svg>
            <input
              bind:this={languageSearchInput}
              bind:value={languageSearch}
              type="search"
              placeholder="Search for any language"
              class="h-full min-w-0 flex-1 bg-transparent text-[14px] text-zinc-100 outline-none placeholder:text-zinc-600"
            />
          </label>

          <div class="scroll-area min-h-0 flex-1 overflow-y-auto p-4">
            {#if filteredLanguages.length > 0}
              <div class="grid grid-cols-2 gap-2.5 lg:grid-cols-3">
                {#each filteredLanguages as language}
                  <button
                    type="button"
                    on:click={() => toggleDraftLanguage(language.id)}
                    aria-pressed={!draftAutoDetect && draftLanguages.includes(language.id)}
                    class="flex h-[58px] min-w-0 items-center gap-3 rounded-lg border px-3 text-left transition {!draftAutoDetect &&
                    draftLanguages.includes(language.id)
                      ? 'border-indigo-400 bg-indigo-500/15 text-zinc-100'
                      : 'border-zinc-800 bg-zinc-900 text-zinc-300 opacity-80 hover:border-zinc-700 hover:bg-zinc-800 hover:opacity-100'}"
                  >
                    {#if language.flagImage}
                      <img
                        src={language.flagImage}
                        alt=""
                        class="h-[21px] w-7 shrink-0 rounded-[3px] object-cover ring-1 ring-inset ring-white/15"
                      />
                    {:else if language.flag}
                      <span
                        class="fi fi-{language.flag} shrink-0 rounded-[3px] text-[21px] ring-1 ring-inset ring-white/15"
                        aria-hidden="true"
                      ></span>
                    {/if}
                    <span class="min-w-0">
                      <span class="block truncate text-[13px] font-semibold">{language.name}</span>
                      <span class="mt-0.5 block truncate text-[11px] text-zinc-500">{language.nativeName}</span>
                    </span>
                  </button>
                {/each}
              </div>
            {:else}
              <div class="flex h-full items-center justify-center text-[13px] text-zinc-500">
                No languages found.
              </div>
            {/if}
          </div>
        </div>

        <aside class="flex min-h-0 flex-col px-5 py-4">
          <h3 class="shrink-0 text-[13px] font-semibold text-zinc-100">Selected</h3>
          <div class="mt-3 min-h-0 flex-1 space-y-1 overflow-y-auto">
            {#if draftAutoDetect}
              <div class="rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-2 text-[12px] font-medium text-zinc-300">
                All supported languages
              </div>
            {:else}
              {#each draftLanguageOptions as language}
                <div class="flex h-9 items-center gap-2 rounded-lg px-1 text-[12px] text-zinc-300">
                  {#if language.flagImage}
                    <img
                      src={language.flagImage}
                      alt=""
                      class="h-[15px] w-5 shrink-0 rounded-[2px] object-cover ring-1 ring-inset ring-white/15"
                    />
                  {:else if language.flag}
                    <span
                      class="fi fi-{language.flag} shrink-0 rounded-[2px] text-[15px] ring-1 ring-inset ring-white/15"
                      aria-hidden="true"
                    ></span>
                  {/if}
                  <span class="min-w-0 flex-1 truncate">{language.name}</span>
                  <button
                    type="button"
                    on:click={() => removeDraftLanguage(language.id)}
                    class="shrink-0 rounded px-1.5 py-1 text-zinc-500 transition-colors hover:bg-zinc-900 hover:text-zinc-200"
                    aria-label={`Remove ${language.name}`}
                  >
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" class="h-3.5 w-3.5" aria-hidden="true">
                      <path d="M5 12h14" />
                    </svg>
                  </button>
                </div>
              {/each}
            {/if}
          </div>
          <div class="flex shrink-0 justify-end gap-2 pt-4">
            <button
              type="button"
              on:click={closeLanguagePicker}
              class="rounded-lg border border-zinc-800 bg-zinc-900 px-4 py-2 text-[12px] font-semibold text-zinc-300 transition-colors hover:bg-zinc-800"
            >
              Cancel
            </button>
            <button
              type="button"
              on:click={saveLanguagePicker}
              disabled={restarting}
              class="rounded-lg bg-zinc-100 px-4 py-2 text-[12px] font-semibold text-zinc-950 transition-colors hover:bg-white disabled:opacity-60"
            >
              Save and close
            </button>
          </div>
        </aside>
      </div>
    </div>
  </div>
{/if}
