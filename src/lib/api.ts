import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

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
}

export const api = {
  startScan: () => invoke("start_scan"),
  cancelScan: () => invoke("cancel_scan"),
  finishScan: (region: Region, dpr: number) =>
    invoke<ScanResult>("finish_scan", { region, dpr }),
  scanImage: (imageB64: string) =>
    invoke<ScanResult>("scan_image", { imageB64 }),
  scanClipboardImage: () =>
    invoke<ScanResult | null>("scan_clipboard_image"),
  getHistory: () => invoke<HistoryEntry[]>("get_history"),
  clearHistory: () => invoke("clear_history"),
  deleteHistoryEntry: (id: number) =>
    invoke("delete_history_entry", { id }),
  copyToClipboard: (content: string) =>
    invoke("copy_to_clipboard", { content }),
  setAutostart: (enabled: boolean) =>
    invoke("set_autostart", { enabled }),
  getAutostart: () => invoke<boolean>("get_autostart"),
  appInfo: () => invoke<AppInfo>("app_info"),
  onScanResult: (cb: (r: ScanResult) => void) =>
    listen<ScanResult>("scan-result", (event) => cb(event.payload)),
};

export function formatTimestamp(ts: number): string {
  const d = new Date(ts * 1000);
  return d.toLocaleString();
}
