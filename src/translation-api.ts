import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export const MAX_SOURCE_LENGTH = 4000;

/** Matches the camelCase payload serialized by pipeline::Progress. */
export interface TranslationProgress {
  selectionId: number;
  requestId: number;
  phase: "recognizing" | "translating" | "done" | "error";
  sourceText: string;
  translation: string;
  error: string | null;
  model: string;
  ocrMs: number | null;
  translationMs: number | null;
  historyId: number | null;
  selectedForStudy: boolean;
  storageError: string | null;
}

interface ModelList {
  models: string[];
  selected: string;
}

export function getTranslation() {
  return invoke<TranslationProgress | null>("get_translation");
}

export function translateText(selectionId: number, text: string) {
  return invoke<void>("translate_text", { selectionId, text });
}

export function cancelTranslation() {
  return invoke<void>("cancel_translation");
}

export function onTranslationProgress(callback: (progress: TranslationProgress) => void) {
  return listen<TranslationProgress>("translation-progress", (event) => callback(event.payload));
}

export function getModels() {
  return invoke<ModelList>("get_models");
}

export function selectModel(model: string) {
  return invoke<void>("select_model", { model });
}
