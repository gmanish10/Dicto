import { invoke } from "@tauri-apps/api/core";

export type SttProvider = "local" | "groq" | "open_ai";
export type PolishProvider =
  | "auto"
  | "apple_intelligence"
  | "bundled_llm"
  | "local_lite"
  | "claude"
  | "groq_llama"
  | "none";

export interface HotkeyBinding {
  chord: string;
}

export interface Settings {
  hotkey: HotkeyBinding;
  stt_provider: SttProvider;
  polish_provider: PolishProvider;
  language: string;
  model_name: string;
  microphone_name: string | null;
  play_start_chime: boolean;
  play_stop_chime: boolean;
  auto_paste: boolean;
  max_recording_seconds: number;
  onboarding_completed: boolean;
  paused: boolean;
  show_recording_overlay: boolean;
  /** Armed onboarding resume marker. Empty normally; set to "permissions"
   *  only when the user initiates an Accessibility / Input-Monitoring
   *  grant, so the macOS-forced relaunch resumes onto Permissions. Any
   *  other launch starts at Welcome. */
  onboarding_step: string;
}

export type PermissionStatus = "granted" | "denied" | "not_determined";

export interface PermissionsSnapshot {
  microphone: PermissionStatus;
  accessibility: PermissionStatus;
  input_monitoring: PermissionStatus;
}

export interface MicrophoneInfo {
  name: string;
  is_default: boolean;
}

export interface TranscriptRow {
  id: number;
  raw: string;
  polished: string;
  ts: number;
  duration_ms: number;
  provider_stt: string;
  provider_polish: string | null;
}

export interface CustomWord {
  id: number;
  word: string;
  weight: number;
  created_at: number;
}

export interface Replacement {
  id: number;
  trigger: string;
  replacement: string;
  case_sensitive: boolean;
}

export type ApiKey = "groq" | "openai" | "anthropic";

export interface ApiKeyStatus {
  key: ApiKey;
  configured: boolean;
}

export interface DownloadProgress {
  bytes: number;
  total: number;
}

export interface BundledLlmStatus {
  downloaded: boolean;
  size_mb: number;
  downloading: DownloadProgress | null;
}

export interface AppleIntelligenceStatus {
  available: boolean;
}

export interface PolishAvailability {
  bundled_llm: BundledLlmStatus;
  apple_intelligence: AppleIntelligenceStatus;
}

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  setSettings: (settings: Settings) => invoke<void>("set_settings", { settings }),
  checkPermissions: () => invoke<PermissionsSnapshot>("check_permissions"),
  requestMicrophonePermission: () => invoke<PermissionStatus>("request_microphone_permission"),
  openSystemSettings: (pane: "microphone" | "accessibility" | "input_monitoring") =>
    invoke<void>("open_system_settings", { pane }),
  listMicrophones: () => invoke<MicrophoneInfo[]>("list_microphones"),
  listHistory: (limit = 20) => invoke<TranscriptRow[]>("list_history", { limit }),
  clearHistory: () => invoke<void>("clear_history"),
  listDictionaryWords: () => invoke<CustomWord[]>("list_dictionary_words"),
  addDictionaryWord: (word: string, weight: number) =>
    invoke<void>("add_dictionary_word", { word, weight }),
  deleteDictionaryWord: (id: number) => invoke<void>("delete_dictionary_word", { id }),
  listReplacements: () => invoke<Replacement[]>("list_replacements"),
  upsertReplacement: (trigger: string, replacement: string, caseSensitive: boolean) =>
    invoke<void>("upsert_replacement", { trigger, replacement, caseSensitive }),
  deleteReplacement: (id: number) => invoke<void>("delete_replacement", { id }),
  getApiKeyStatus: () => invoke<ApiKeyStatus[]>("get_api_key_status"),
  setApiKey: (key: ApiKey, value: string) => invoke<void>("set_api_key", { key, value }),
  deleteApiKey: (key: ApiKey) => invoke<void>("delete_api_key", { key }),
  setHotkey: (chord: string) => invoke<void>("set_hotkey", { chord }),
  pauseDictation: () => invoke<void>("pause_dictation"),
  resumeDictation: () => invoke<void>("resume_dictation"),
  recheckForUpdates: () => invoke<string | null>("recheck_for_updates"),
  installPendingUpdate: () => invoke<void>("install_pending_update"),
  checkPolishAvailability: () => invoke<PolishAvailability>("check_polish_availability"),
  startPolishModelDownload: () => invoke<void>("start_polish_model_download"),
  reinjectTranscript: (id: number) => invoke<void>("reinject_transcript", { id }),
  recordCorrection: (raw: string, finalText: string) =>
    invoke<void>("record_correction", { raw, finalText }),
  openMainWindow: () => invoke<void>("open_main_window"),
  finishOnboarding: () => invoke<void>("finish_onboarding"),
  startRuntime: () => invoke<void>("start_runtime"),
};
