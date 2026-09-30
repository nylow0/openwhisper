// Spoken-language options for the language picker: display names, native names,
// and flags. Flags come from flag-icons classes, or custom SVGs for regions that
// have no ISO country code.
import { SUPPORTED_ASR_LANGUAGES, type AsrLanguageCode } from '../shared/types';
import bashkortostanFlag from './flags/bashkortostan.svg';
import tatarstanFlag from './flags/tatarstan.svg';

export type LanguageOption = {
  id: AsrLanguageCode;
  name: string;
  nativeName: string;
  /** flag-icons class suffix, e.g. "ua" renders `fi fi-ua`. Empty when unknown. */
  flag: string;
  /** Custom flag image; takes precedence over `flag`. */
  flagImage?: string;
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

/** Converts a flag emoji (two regional-indicator symbols) to its ISO 3166-1 code. */
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

function nativeLanguageName(language: AsrLanguageCode, fallback: string): string {
  try {
    const name = new Intl.DisplayNames([language], { type: 'language' }).of(language);
    // Intl echoes the code back (e.g. "ba") when it has no native name.
    if (!name || name.toLowerCase() === language) return fallback;
    return name.charAt(0).toLocaleUpperCase(language) + name.slice(1);
  } catch {
    return fallback;
  }
}

/** Every supported language, sorted by English name. */
export const LANGUAGES: LanguageOption[] = [...SUPPORTED_ASR_LANGUAGES]
  .sort((a, b) => a.name.localeCompare(b.name))
  .map((language) => {
    const nativeName = nativeLanguageName(language.id, language.name);
    return {
      id: language.id,
      name: language.name,
      nativeName,
      flag: FLAG_CODE_OVERRIDES[language.id] ?? flagEmojiToCode(LANGUAGE_FLAGS[language.id]),
      flagImage: FLAG_IMAGE_OVERRIDES[language.id],
      searchText: `${language.name} ${nativeName} ${language.id}`.toLowerCase(),
    };
  });

export const LANGUAGE_BY_ID = new Map<string, LanguageOption>(LANGUAGES.map((language) => [language.id, language]));

/** Resolves language codes to options, skipping unknown codes. */
export function languageOptions(ids: readonly string[]): LanguageOption[] {
  return ids.flatMap((id) => LANGUAGE_BY_ID.get(id) ?? []);
}

/** Matches English name, native name, or code (case-insensitive). */
export function searchLanguages(query: string): LanguageOption[] {
  const needle = query.trim().toLowerCase();
  return needle ? LANGUAGES.filter((language) => language.searchText.includes(needle)) : LANGUAGES;
}
