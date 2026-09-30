// Unsaved edits in the language picker. Pure functions so the picker rules can be
// tested without a DOM. The draft is applied to settings only when the user hits Apply.
import type { AppSettings, AsrLanguageCode } from '../shared/types';

export type LanguageDraft = {
  languages: AsrLanguageCode[];
  /** Auto-detect across every supported language; `languages` is kept for when it turns off. */
  auto: boolean;
};

export function draftFromSettings(settings: AppSettings): LanguageDraft {
  return { languages: [...settings.spokenLanguages], auto: settings.autoDetectLanguage };
}

/**
 * Picking a language while auto-detect is on switches to just that language.
 * Unselecting the last language falls back to auto-detect instead of leaving none.
 */
export function toggleDraftLanguage(draft: LanguageDraft, id: AsrLanguageCode): LanguageDraft {
  if (draft.auto) return { languages: [id], auto: false };
  const selected = draft.languages.includes(id);
  if (selected && draft.languages.length === 1) return { ...draft, auto: true };
  return {
    auto: false,
    languages: selected ? draft.languages.filter((language) => language !== id) : [...draft.languages, id],
  };
}

export function isDraftDirty(draft: LanguageDraft, settings: AppSettings): boolean {
  return (
    draft.auto !== settings.autoDetectLanguage ||
    draft.languages.length !== settings.spokenLanguages.length ||
    draft.languages.some((language, index) => language !== settings.spokenLanguages[index])
  );
}

/** Settings patch for Apply. Language choice only exists on the multilingual model. */
export function draftToSettings(draft: LanguageDraft, current: AppSettings): Partial<AppSettings> {
  return {
    model: 'large_v3_turbo_q8',
    spokenLanguages: draft.languages.length > 0 ? draft.languages : current.spokenLanguages,
    autoDetectLanguage: draft.auto,
  };
}

/** Moves `pinned` items to the front, keeping the original order within each group. */
export function pinFirst<T extends { id: string }>(items: T[], pinned: readonly string[]): T[] {
  return [...items.filter((item) => pinned.includes(item.id)), ...items.filter((item) => !pinned.includes(item.id))];
}
