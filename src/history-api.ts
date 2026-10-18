import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/** JSON representation of history::Entry from the Rust backend. */
export interface HistoryEntry {
  id: number;
  sourceText: string;
  translation: string;
  model: string;
  createdAt: string;
  updatedAt: string;
  selectedForStudy: boolean;
}

/** Counts describe the query and all study bookmarks, not just this page. */
export interface HistoryPage {
  entries: HistoryEntry[];
  total: number;
  selectedCount: number;
  page: number;
  pageSize: number;
  storagePath: string;
}

interface AnkiExport {
  path: string;
  count: number;
}

export function queryHistory(page: number, search: string, onlyStudy: boolean) {
  return invoke<HistoryPage>("query_history", { page, search, onlyStudy });
}

/** Rust broadcasts the update to both the history and translation windows. */
export function setStudyFlag(id: number, selected: boolean) {
  return invoke<void>("set_study_flag", { id, selected });
}

/** A null result means the native save dialog was cancelled. */
export function exportAnki() {
  return invoke<AnkiExport | null>("export_anki");
}

export function onHistoryChanged(callback: () => void) {
  return listen("history-changed", callback);
}
