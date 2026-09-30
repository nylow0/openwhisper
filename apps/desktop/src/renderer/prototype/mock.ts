// PROTOTYPE: in-memory state and sample data shared by every design variant.
// Nothing here touches window.api; the prototype runs in a plain browser tab.
import { derived, writable } from 'svelte/store';
import {
  SUPPORTED_ASR_LANGUAGES,
  type AppSettings,
  type AsrDevice,
  type AsrLanguageCode,
  type AsrModel,
  type HistoryItem,
} from '../../shared/types';
import { LANGUAGE_FLAGS } from './flags';
import bashkortostanFlag from '../flags/bashkortostan.svg';
import tatarstanFlag from '../flags/tatarstan.svg';

export const MODELS: Array<{ id: AsrModel; name: string; detail: string; size: string }> = [
  { id: 'large_v3_turbo_q8', name: 'Multilingual', detail: 'Best accuracy, 99 languages', size: '874 MB' },
  { id: 'medium_en_q8', name: 'English only', detail: 'Smaller fallback for English', size: '823 MB' },
];

export const DEVICES: Array<{ id: AsrDevice; name: string }> = [
  { id: 'auto', name: 'Auto' },
  { id: 'cpu', name: 'CPU' },
  { id: 'gpu', name: 'GPU' },
];

export type LanguageOption = {
  id: AsrLanguageCode;
  name: string;
  nativeName: string;
  /** flag-icons class suffix, e.g. "ua" for `fi fi-ua`. */
  flag: string;
  flagImage?: string;
  searchText: string;
};

function nativeLanguageName(language: AsrLanguageCode, fallback: string): string {
  try {
    const name = new Intl.DisplayNames([language], { type: 'language' }).of(language);
    // Intl echoes the code back ("ba") when it has no native name.
    if (!name || name.toLowerCase() === language) return fallback;
    return name.charAt(0).toLocaleUpperCase(language) + name.slice(1);
  } catch {
    return fallback;
  }
}

function flagEmojiToCode(flag: string): string {
  const chars = [...flag];
  if (chars.length !== 2) return '';
  return chars.map((char) => String.fromCharCode((char.codePointAt(0) ?? 0) - 0x1f1e6 + 0x61)).join('');
}

export const LANGUAGES: LanguageOption[] = [...SUPPORTED_ASR_LANGUAGES]
  .sort((a, b) => a.name.localeCompare(b.name))
  .map((language) => {
    const nativeName = nativeLanguageName(language.id, language.name);
    return {
      id: language.id,
      name: language.name,
      nativeName,
      flag: language.id === 'cy' ? 'gb-wls' : flagEmojiToCode(LANGUAGE_FLAGS[language.id]),
      flagImage: language.id === 'ba' ? bashkortostanFlag : language.id === 'tt' ? tatarstanFlag : undefined,
      searchText: `${language.name} ${nativeName} ${language.id}`.toLowerCase(),
    };
  });

export const LANGUAGE_BY_ID = new Map(LANGUAGES.map((language) => [language.id, language]));

export function languageOptions(ids: AsrLanguageCode[]): LanguageOption[] {
  return ids.flatMap((id) => LANGUAGE_BY_ID.get(id) ?? []);
}

/** Filters by English name, native name, or code; `pinned` languages are listed first. */
export function filterLanguages(query: string, pinned: AsrLanguageCode[] = []): LanguageOption[] {
  const needle = query.trim().toLowerCase();
  const matches = needle ? LANGUAGES.filter((language) => language.searchText.includes(needle)) : LANGUAGES;
  return [
    ...matches.filter((language) => pinned.includes(language.id)),
    ...matches.filter((language) => !pinned.includes(language.id)),
  ];
}

// ─── Language picker draft rules (same behaviour as the real Settings picker) ───

export type LanguageDraft = { languages: AsrLanguageCode[]; auto: boolean };

/** Picking a language exits auto-detect; unselecting the last one falls back to auto-detect. */
export function toggleDraftLanguage(draft: LanguageDraft, id: AsrLanguageCode): LanguageDraft {
  if (draft.auto) return { languages: [id], auto: false };
  const selected = draft.languages.includes(id);
  if (selected && draft.languages.length === 1) return { ...draft, auto: true };
  return {
    auto: false,
    languages: selected ? draft.languages.filter((language) => language !== id) : [...draft.languages, id],
  };
}

export function draftFromSettings(value: AppSettings): LanguageDraft {
  return { languages: [...value.spokenLanguages], auto: value.autoDetectLanguage };
}

export function saveDraft(draft: LanguageDraft) {
  settings.update((value) => ({
    ...value,
    model: 'large_v3_turbo_q8',
    spokenLanguages: draft.languages.length > 0 ? draft.languages : value.spokenLanguages,
    autoDetectLanguage: draft.auto,
  }));
  restartEngine();
}

// ─── Settings + engine ───

export const settings = writable<AppSettings>({
  model: 'large_v3_turbo_q8',
  device: 'auto',
  spokenLanguages: ['en', 'uk'],
  autoDetectLanguage: false,
  launchAtLogin: true,
  showWindowOnLaunch: false,
});

export const restarting = writable(false);

/** Fake engine restart: flips `restarting` for a moment, like the real IPC round trip. */
export function restartEngine() {
  restarting.set(true);
  setTimeout(() => restarting.set(false), 1400);
}

export function patchSettings(partial: Partial<AppSettings>) {
  const engineChange = 'model' in partial || 'device' in partial;
  settings.update((value) => ({ ...value, ...partial }));
  if (engineChange) restartEngine();
}

export type StartupToggle = 'launchAtLogin' | 'showWindowOnLaunch';

export function setStartupToggle(key: StartupToggle, value: boolean) {
  patchSettings(key === 'launchAtLogin' ? { launchAtLogin: value } : { showWindowOnLaunch: value });
}

// ─── History ───

const MINUTE = 60_000;
const now = Date.now();
const startOfToday = new Date().setHours(0, 0, 0, 0);

const SAMPLE_HISTORY: HistoryItem[] = [
  {
    id: 'h1',
    text: "Hey, I pushed the fix for the overlay flicker. Can you pull main and check if it still happens on your second monitor?",
    language: 'en',
    latencyMs: 312,
    createdAt: now - 4 * MINUTE,
  },
  {
    id: 'h2',
    text: 'Привіт! Я буду трохи пізніше, десь о пів на сьому. Візьми, будь ласка, хліб по дорозі.',
    language: 'uk',
    latencyMs: 405,
    createdAt: now - 38 * MINUTE,
  },
  {
    id: 'h3',
    text: 'Refactor the transcript store so history writes are batched. Right now every final event triggers a full JSON rewrite, which is fine for a hundred items but gets slow past a few thousand.',
    language: 'en',
    latencyMs: 488,
    createdAt: now - 95 * MINUTE,
  },
  {
    id: 'h4',
    text: 'Remind me to email the professor about the lab report deadline.',
    language: 'en',
    latencyMs: 241,
    createdAt: now - 3 * 60 * MINUTE,
  },
  {
    id: 'h5',
    text: 'Домашнє завдання з фізики: параграф дванадцять, задачі з третьої по сьому, і лабораторна до п’ятниці.',
    language: 'uk',
    latencyMs: 522,
    createdAt: startOfToday - 5 * 60 * MINUTE,
  },
  {
    id: 'h6',
    text: 'The benchmark on LibriSpeech test-other came in at 4.1 percent word error rate on GPU, about 0.3 seconds per utterance.',
    language: 'en',
    latencyMs: 367,
    createdAt: startOfToday - 7 * 60 * MINUTE,
  },
  {
    id: 'h7',
    text: 'Thanks for the review. I agree the settings page is too busy, let me try a couple of simpler layouts and send you screenshots.',
    language: 'en',
    latencyMs: 298,
    createdAt: startOfToday - 9 * 60 * MINUTE,
  },
  {
    id: 'h8',
    text: 'Купити: молоко, яйця, гречку, яблука.',
    language: 'uk',
    latencyMs: 214,
    createdAt: startOfToday - 30 * 60 * MINUTE,
  },
  {
    id: 'h9',
    text: 'Meeting notes. We decided to ship the Windows build first, keep macOS for later, and drop the streaming partials until latency is under two hundred milliseconds.',
    language: 'en',
    latencyMs: 611,
    createdAt: startOfToday - 4 * 24 * 60 * MINUTE,
  },
];

export const history = writable<HistoryItem[]>(SAMPLE_HISTORY);

export function setHistorySample(empty: boolean) {
  history.set(empty ? [] : SAMPLE_HISTORY);
}

export function deleteHistoryItem(id: string) {
  history.update((items) => items.filter((item) => item.id !== id));
}

export function clearHistory() {
  history.set([]);
}

export async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

export const stats = derived(history, (items) => {
  const latencies = items.flatMap((item) => (item.latencyMs === null ? [] : [item.latencyMs]));
  return {
    count: items.length,
    words: items.reduce((sum, item) => sum + item.text.split(/\s+/).filter(Boolean).length, 0),
    avgLatencyMs: latencies.length ? Math.round(latencies.reduce((a, b) => a + b, 0) / latencies.length) : null,
  };
});

export function formatTime(ms: number) {
  return new Date(ms).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

export function formatWhen(ms: number) {
  const date = new Date(ms);
  if (date.toDateString() === new Date().toDateString()) return formatTime(ms);
  return `${date.toLocaleDateString([], { month: 'short', day: 'numeric' })} · ${formatTime(ms)}`;
}

export function dayLabel(ms: number) {
  const day = new Date(ms).setHours(0, 0, 0, 0);
  if (day === startOfToday) return 'Today';
  if (day === startOfToday - 24 * 60 * MINUTE) return 'Yesterday';
  return new Date(ms).toLocaleDateString([], { weekday: 'long', month: 'short', day: 'numeric' });
}

/** Groups already-sorted (newest first) items into consecutive day buckets. */
export function groupByDay(items: HistoryItem[]): Array<{ label: string; items: HistoryItem[] }> {
  const groups: Array<{ label: string; items: HistoryItem[] }> = [];
  for (const item of items) {
    const label = dayLabel(item.createdAt);
    const last = groups[groups.length - 1];
    if (last?.label === label) last.items.push(item);
    else groups.push({ label, items: [item] });
  }
  return groups;
}

export function wordCount(text: string) {
  return text.split(/\s+/).filter(Boolean).length;
}

// ─── Toasts ───

export type Toast = { id: number; code: string; message: string };
export const toasts = writable<Toast[]>([]);
let nextToastId = 1;

export function pushToast(code: string, message: string) {
  const id = nextToastId++;
  toasts.update((list) => [{ id, code, message }, ...list].slice(0, 3));
  setTimeout(() => dismissToast(id), 6000);
}

export function dismissToast(id: number) {
  toasts.update((list) => list.filter((toast) => toast.id !== id));
}

export function simulateError() {
  pushToast('ENGINE_CRASHED', 'Lost connection to the speech engine. Restart it from Settings.');
}

// ─── Overlay ───

export type OverlayPhase = 'recording' | 'transcribing' | 'done' | 'error';
export const OVERLAY_PHASES: OverlayPhase[] = ['recording', 'transcribing', 'done', 'error'];

export type OverlayProps = {
  phase: OverlayPhase;
  elapsedMs: number;
  partial: string;
  finalText: string;
  errorText: string;
};

export const OVERLAY_SAMPLE = {
  partial: 'so the plan for tomorrow is to finish the',
  finalText: 'So the plan for tomorrow is to finish the settings redesign and send screenshots.',
  errorText: 'Microphone is unavailable. Check that no other app is using it.',
};

export function formatElapsed(ms: number) {
  const total = Math.floor(ms / 1000);
  return `${Math.floor(total / 60)}:${(total % 60).toString().padStart(2, '0')}`;
}
