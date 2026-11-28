import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { withTimeout } from "./timeout";

export interface Region {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface ScanResult {
  kind: string;
  content: string;
  format: string;
  preview: string | null;
  timestamp: number;
  copied: boolean;
}

export interface HistoryEntry {
  id: number;
  created_at: number;
  kind: string;
  content: string;
  format: string;
  preview: string | null;
}

export interface AppInfo {
  version: string;
  shortcut: string;
  ocr_ready: boolean;
  ocr_error: string | null;
}

const QUICK_TIMEOUT = 10_000;
const SCAN_TIMEOUT = 30_000;

function call<T>(command: string, args?: Record<string, unknown>, timeout = QUICK_TIMEOUT) {
  return withTimeout(invoke<T>(command, args), timeout, command);
}

export const api = {
  startScan: () => call("start_scan"),
  cancelScan: () => call("cancel_scan"),
  finishScan: (region: Region, dpr: number) =>
    call<ScanResult>("finish_scan", { region, dpr }, SCAN_TIMEOUT),
  scanImage: (imageB64: string) =>
    call<ScanResult>("scan_image", { imageB64 }, SCAN_TIMEOUT),
  scanClipboardImage: () =>
    call<ScanResult | null>("scan_clipboard_image", undefined, SCAN_TIMEOUT),
  getHistory: () => call<HistoryEntry[]>("get_history"),
  getHistoryPreview: (id: number) =>
    call<string | null>("get_history_preview", { id }),
  clearHistory: () => call("clear_history"),
  deleteHistoryEntry: (id: number) => call("delete_history_entry", { id }),
  copyToClipboard: (content: string) => call("copy_to_clipboard", { content }),
  setAutostart: (enabled: boolean) => call("set_autostart", { enabled }),
  getAutostart: () => call<boolean>("get_autostart"),
  appInfo: () => call<AppInfo>("app_info"),
  onScanResult: (cb: (r: ScanResult) => void) =>
    listen<ScanResult>("scan-result", (event) => cb(event.payload)),
  onScanError: (cb: (message: string) => void) =>
    listen<string>("scan-error", (event) => cb(event.payload)),
  onScanBusy: (cb: (busy: boolean) => void) =>
    listen<boolean>("scan-busy", (event) => cb(event.payload)),
};

export function formatTimestamp(ts: number): string {
  const d = new Date(ts * 1000);
  return d.toLocaleString();
}
