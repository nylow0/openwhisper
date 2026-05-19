// Shared event and app data types for OpenWhisper.

export interface WordResult {
  text: string;
  startMs: number;
  endMs: number;
  confidence?: number;
}

export interface DictationStartedEvent {
  type: 'dictation.started';
  timestamp: number;
}

export interface DictationStoppedEvent {
  type: 'dictation.stopped';
  timestamp: number;
}

export interface TranscriptPartialEvent {
  type: 'transcript.partial';
  text: string;
  isFinal: false;
  processingLatencyMs?: number;
}

export interface TranscriptFinalEvent {
  type: 'transcript.final';
  text: string;
  words: WordResult[];
  language?: string | null;
  processingLatencyMs?: number;
}

export interface StatusEvent {
  type: 'status';
  isDictating: boolean;
  isModelLoaded: boolean;
  workerHealthy: boolean;
}

export interface ErrorEvent {
  type: 'error';
  code: string;
  message: string;
  recoverable: boolean;
  details?: Record<string, unknown>;
}

export type Event =
  | DictationStartedEvent
  | DictationStoppedEvent
  | TranscriptPartialEvent
  | TranscriptFinalEvent
  | StatusEvent
  | ErrorEvent;

// ─── App-level settings & history (Electron <-> renderer) ───

export type AsrModel = 'medium_en_q8' | 'large_v3_turbo_q8';
export type AsrDevice = 'auto' | 'cpu' | 'gpu';
export const SUPPORTED_ASR_LANGUAGES = [
  { id: 'en', name: 'English' },
  { id: 'zh', name: 'Chinese' },
  { id: 'de', name: 'German' },
  { id: 'es', name: 'Spanish' },
  { id: 'ru', name: 'Russian' },
  { id: 'ko', name: 'Korean' },
  { id: 'fr', name: 'French' },
  { id: 'ja', name: 'Japanese' },
  { id: 'pt', name: 'Portuguese' },
  { id: 'tr', name: 'Turkish' },
  { id: 'pl', name: 'Polish' },
  { id: 'ca', name: 'Catalan' },
  { id: 'nl', name: 'Dutch' },
  { id: 'ar', name: 'Arabic' },
  { id: 'sv', name: 'Swedish' },
  { id: 'it', name: 'Italian' },
  { id: 'id', name: 'Indonesian' },
  { id: 'hi', name: 'Hindi' },
  { id: 'fi', name: 'Finnish' },
  { id: 'vi', name: 'Vietnamese' },
  { id: 'he', name: 'Hebrew' },
  { id: 'uk', name: 'Ukrainian' },
  { id: 'el', name: 'Greek' },
  { id: 'ms', name: 'Malay' },
  { id: 'cs', name: 'Czech' },
  { id: 'ro', name: 'Romanian' },
  { id: 'da', name: 'Danish' },
  { id: 'hu', name: 'Hungarian' },
  { id: 'ta', name: 'Tamil' },
  { id: 'no', name: 'Norwegian' },
  { id: 'th', name: 'Thai' },
  { id: 'ur', name: 'Urdu' },
  { id: 'hr', name: 'Croatian' },
  { id: 'bg', name: 'Bulgarian' },
  { id: 'lt', name: 'Lithuanian' },
  { id: 'la', name: 'Latin' },
  { id: 'mi', name: 'Maori' },
  { id: 'ml', name: 'Malayalam' },
  { id: 'cy', name: 'Welsh' },
  { id: 'sk', name: 'Slovak' },
  { id: 'te', name: 'Telugu' },
  { id: 'fa', name: 'Persian' },
  { id: 'lv', name: 'Latvian' },
  { id: 'bn', name: 'Bengali' },
  { id: 'sr', name: 'Serbian' },
  { id: 'az', name: 'Azerbaijani' },
  { id: 'sl', name: 'Slovenian' },
  { id: 'kn', name: 'Kannada' },
  { id: 'et', name: 'Estonian' },
  { id: 'mk', name: 'Macedonian' },
  { id: 'br', name: 'Breton' },
  { id: 'eu', name: 'Basque' },
  { id: 'is', name: 'Icelandic' },
  { id: 'hy', name: 'Armenian' },
  { id: 'ne', name: 'Nepali' },
  { id: 'mn', name: 'Mongolian' },
  { id: 'bs', name: 'Bosnian' },
  { id: 'kk', name: 'Kazakh' },
  { id: 'sq', name: 'Albanian' },
  { id: 'sw', name: 'Swahili' },
  { id: 'gl', name: 'Galician' },
  { id: 'mr', name: 'Marathi' },
  { id: 'pa', name: 'Punjabi' },
  { id: 'si', name: 'Sinhala' },
  { id: 'km', name: 'Khmer' },
  { id: 'sn', name: 'Shona' },
  { id: 'yo', name: 'Yoruba' },
  { id: 'so', name: 'Somali' },
  { id: 'af', name: 'Afrikaans' },
  { id: 'oc', name: 'Occitan' },
  { id: 'ka', name: 'Georgian' },
  { id: 'be', name: 'Belarusian' },
  { id: 'tg', name: 'Tajik' },
  { id: 'sd', name: 'Sindhi' },
  { id: 'gu', name: 'Gujarati' },
  { id: 'am', name: 'Amharic' },
  { id: 'yi', name: 'Yiddish' },
  { id: 'lo', name: 'Lao' },
  { id: 'uz', name: 'Uzbek' },
  { id: 'fo', name: 'Faroese' },
  { id: 'ht', name: 'Haitian Creole' },
  { id: 'ps', name: 'Pashto' },
  { id: 'tk', name: 'Turkmen' },
  { id: 'nn', name: 'Nynorsk' },
  { id: 'mt', name: 'Maltese' },
  { id: 'sa', name: 'Sanskrit' },
  { id: 'lb', name: 'Luxembourgish' },
  { id: 'my', name: 'Myanmar' },
  { id: 'bo', name: 'Tibetan' },
  { id: 'tl', name: 'Tagalog' },
  { id: 'mg', name: 'Malagasy' },
  { id: 'as', name: 'Assamese' },
  { id: 'tt', name: 'Tatar' },
  { id: 'haw', name: 'Hawaiian' },
  { id: 'ln', name: 'Lingala' },
  { id: 'ha', name: 'Hausa' },
  { id: 'ba', name: 'Bashkir' },
  { id: 'jw', name: 'Javanese' },
  { id: 'su', name: 'Sundanese' },
  { id: 'yue', name: 'Cantonese' },
] as const;
export type AsrLanguageCode = (typeof SUPPORTED_ASR_LANGUAGES)[number]['id'];

export interface AppSettings {
  /** whisper.cpp model key passed to the ASR worker. */
  model: AsrModel;
  /** Compute device preference for transcription. */
  device: AsrDevice;
  /** Spoken languages enabled for transcription. */
  spokenLanguages: AsrLanguageCode[];
  /** Start OpenWhisper automatically when the user signs in. */
  launchAtLogin: boolean;
  /** Open the main window on launch (vs. starting only in the tray). */
  showWindowOnLaunch: boolean;
}

export interface HistoryItem {
  id: string;
  text: string;
  language: string | null;
  latencyMs: number | null;
  /** Epoch milliseconds. */
  createdAt: number;
}
