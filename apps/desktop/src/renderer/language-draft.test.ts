// Walks the language picker through the flows a user actually takes: open with the
// saved languages, add/remove some, fall back to auto-detect, and Apply.
import { describe, expect, test } from 'bun:test';
import type { AppSettings } from '../shared/types';
import { draftFromSettings, draftToSettings, isDraftDirty, pinFirst, toggleDraftLanguage } from './language-draft';

const saved: AppSettings = {
  model: 'large_v3_turbo_q8',
  device: 'auto',
  spokenLanguages: ['en', 'uk'],
  autoDetectLanguage: false,
  launchAtLogin: true,
  showWindowOnLaunch: false,
};

describe('language picker draft', () => {
  test('adding a language marks the draft dirty and Apply saves it', () => {
    const draft = toggleDraftLanguage(draftFromSettings(saved), 'de');
    expect(draft).toEqual({ languages: ['en', 'uk', 'de'], auto: false });
    expect(isDraftDirty(draft, saved)).toBe(true);
    expect(draftToSettings(draft, saved)).toEqual({
      model: 'large_v3_turbo_q8',
      spokenLanguages: ['en', 'uk', 'de'],
      autoDetectLanguage: false,
    });
  });

  test('toggling a language off and back on is not a change', () => {
    let draft = toggleDraftLanguage(draftFromSettings(saved), 'uk');
    draft = toggleDraftLanguage(draft, 'uk');
    expect(isDraftDirty(draft, saved)).toBe(false);
  });

  test('removing the last language falls back to auto-detect, never to none', () => {
    let draft = toggleDraftLanguage(draftFromSettings(saved), 'uk');
    draft = toggleDraftLanguage(draft, 'en');
    expect(draft).toEqual({ languages: ['en'], auto: true });
    expect(draftToSettings(draft, saved).spokenLanguages).toEqual(['en']);
  });

  test('picking a language while auto-detect is on switches to just that language', () => {
    const draft = toggleDraftLanguage({ languages: ['en', 'uk'], auto: true }, 'fr');
    expect(draft).toEqual({ languages: ['fr'], auto: false });
  });

  test('Apply from the English-only model switches to the multilingual model', () => {
    const englishOnly: AppSettings = { ...saved, model: 'medium_en_q8' };
    const patch = draftToSettings(toggleDraftLanguage(draftFromSettings(englishOnly), 'de'), englishOnly);
    expect(patch.model).toBe('large_v3_turbo_q8');
  });

  test('saved languages are listed first so they are visible without scrolling', () => {
    const items = [{ id: 'af' }, { id: 'en' }, { id: 'fr' }, { id: 'uk' }];
    expect(pinFirst(items, ['uk', 'en']).map((item) => item.id)).toEqual(['en', 'uk', 'af', 'fr']);
  });
});
