import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface Frame { id: number; width: number; height: number; imageUrl: string }
export interface Selection { id: number; width: number; height: number; imageUrl: string }
export interface Status { shortcut: string; shortcutError: string | null }
export const nativeAvailable = isTauri();
export const startCapture = () => invoke<void>("start_capture");
export const dismissCapture = () => invoke<void>("dismiss_capture");
export const getFrame = () => invoke<Frame | null>("get_capture");
export const getSelection = () => invoke<Selection | null>("get_selection");
export const getStatus = () => invoke<Status>("get_status");
export const showCapture = (frameId: number) => invoke<void>("show_capture", { frameId });
export const submitSelection = (frameId: number, rect: { x: number; y: number; width: number; height: number }) => invoke<void>("submit_selection", { frameId, ...rect });
export const onCapture = (callback: () => void) => listen("capture-started", callback);
export const onSelection = (callback: (selection: Selection) => void) => listen<Selection>("selection-ready", event => callback(event.payload));
export const onCaptureError = (callback: (message: string) => void) => listen<string>("capture-error", event => callback(event.payload));
export const errorMessage = (error: unknown) => error instanceof Error ? error.message : String(error);
