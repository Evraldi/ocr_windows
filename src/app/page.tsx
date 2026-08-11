"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api, formatTimestamp } from "@/lib/api";
import type { AppInfo, HistoryEntry, ScanResult } from "@/lib/api";

type Toast = { text: string; ok: boolean } | null;

export default function Home() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [search, setSearch] = useState("");
  const [toast, setToast] = useState<Toast>(null);
  const [last, setLast] = useState<ScanResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [autoStart, setAutoStart] = useState(false);
  const fileRef = useRef<HTMLInputElement>(null);

  const refresh = useCallback(async () => {
    try {
      setHistory(await api.getHistory());
    } catch (e) {
      setToast({ text: String(e), ok: false });
    }
  }, []);

  const handleResult = useCallback(
    (result: ScanResult | null) => {
      if (!result) {
        setToast({ text: "No image in clipboard", ok: false });
        return;
      }
      setLast(result);
      if (result.kind !== "none") {
        setToast({ text: `${result.format} copied to clipboard`, ok: true });
      } else {
        setToast({ text: result.content, ok: false });
      }
      refresh();
    },
    [refresh]
  );

  useEffect(() => {
    api.appInfo().then(setInfo).catch((e) => setToast({ text: String(e), ok: false }));
    api
      .getAutostart()
      .then(setAutoStart)
      .catch(() => undefined);
    const load = async () => {
      try {
        setHistory(await api.getHistory());
      } catch (e) {
        setToast({ text: String(e), ok: false });
      }
    };
    load();
    const unlisten = api.onScanResult(handleResult);
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [handleResult]);

  const fileToB64 = (file: File): Promise<string> =>
    new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => {
        const url = reader.result as string;
        const idx = url.indexOf(",");
        resolve(idx === -1 ? url : url.slice(idx + 1));
      };
      reader.onerror = () => reject(reader.error ?? new Error("read failed"));
      reader.readAsDataURL(file);
    });

  useEffect(() => {
    const onPaste = (e: ClipboardEvent) => {
      const file = Array.from(e.clipboardData?.files ?? []).find((f) =>
        f.type.startsWith("image/")
      );
      if (!file) return;
      e.preventDefault();
      setBusy(true);
      setToast({ text: "Scanning image…", ok: true });
      fileToB64(file)
        .then((b64) => api.scanImage(b64))
        .then(handleResult)
        .catch((err) => setToast({ text: String(err), ok: false }))
        .finally(() => setBusy(false));
    };
    window.addEventListener("paste", onPaste);
    return () => window.removeEventListener("paste", onPaste);
  }, [handleResult]);

  const onPickImage = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    e.target.value = "";
    if (!file) return;
    setBusy(true);
    setToast({ text: "Scanning image…", ok: true });
    try {
      handleResult(await api.scanImage(await fileToB64(file)));
    } catch (err) {
      setToast({ text: String(err), ok: false });
    } finally {
      setBusy(false);
    }
  };

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return history;
    return history.filter(
      (h) =>
        h.content.toLowerCase().includes(q) ||
        h.format.toLowerCase().includes(q)
    );
  }, [history, search]);

  const copy = async (content: string) => {
    try {
      await api.copyToClipboard(content);
      setToast({ text: "Copied to clipboard", ok: true });
    } catch (e) {
      setToast({ text: String(e), ok: false });
    }
  };

  const clear = async () => {
    try {
      await api.clearHistory();
      setHistory([]);
      setToast({ text: "History cleared", ok: true });
    } catch (e) {
      setToast({ text: String(e), ok: false });
    }
  };

  const removeEntry = async (id: number) => {
    try {
      await api.deleteHistoryEntry(id);
      setHistory((h) => h.filter((x) => x.id !== id));
      setToast({ text: "Entry deleted", ok: true });
    } catch (e) {
      setToast({ text: String(e), ok: false });
    }
  };

  const toggleAutoStart = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const on = e.target.checked;
    try {
      await api.setAutostart(on);
      setAutoStart(on);
      setToast({
        text: on ? "Will start with Windows on login" : "Start with Windows disabled",
        ok: true,
      });
    } catch (err) {
      setToast({ text: String(err), ok: false });
    }
  };

  return (
    <main className="flex h-screen flex-col bg-zinc-950 text-zinc-100">
      <header className="flex items-center justify-between border-b border-zinc-800 px-5 py-3">
        <div>
          <h1 className="text-lg font-semibold tracking-tight">OCR Scan Tool</h1>
          <p className="text-xs text-zinc-500">
            {info ? (
              <>
                Shortcut{" "}
                <kbd className="rounded bg-zinc-800 px-1.5 py-0.5 text-[11px] text-zinc-300">
                  {info.shortcut}
                </kbd>
                {" · "}v{info.version}
              </>
            ) : (
              "loading…"
            )}
          </p>
          <label className="mt-0.5 flex w-fit cursor-pointer items-center gap-1.5 text-xs text-zinc-400">
            <input
              type="checkbox"
              checked={autoStart}
              onChange={toggleAutoStart}
              className="h-3.5 w-3.5 cursor-pointer accent-emerald-500"
            />
            Start with Windows
          </label>
        </div>
        <div className="flex items-center gap-2">
          <input
            ref={fileRef}
            type="file"
            accept="image/*"
            className="hidden"
            onChange={onPickImage}
          />
          <button
            onClick={() => fileRef.current?.click()}
            disabled={busy}
            className="rounded-lg border border-zinc-700 px-3 py-2 text-sm text-zinc-300 transition-colors hover:bg-zinc-800 disabled:opacity-40"
            title="Scan an image file (or press Ctrl+V to paste one)"
          >
            Import Image
          </button>
          <button
            onClick={() => api.startScan().catch((e) => setToast({ text: String(e), ok: false }))}
            className="rounded-lg bg-emerald-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-emerald-500"
          >
            Scan Region
          </button>
          <button
            onClick={clear}
            disabled={history.length === 0}
            className="rounded-lg border border-zinc-700 px-3 py-2 text-sm text-zinc-300 transition-colors hover:bg-zinc-800 disabled:opacity-40"
          >
            Clear
          </button>
        </div>
      </header>

      {toast && (
        <div
          className={`px-5 py-2 text-sm ${
            toast.ok
              ? "bg-emerald-900/40 text-emerald-200"
              : "bg-red-900/40 text-red-200"
          }`}
        >
          {toast.text}
        </div>
      )}

      {last && last.kind !== "none" && (
        <section className="border-b border-zinc-800 bg-zinc-900/60 px-5 py-3">
          <div className="flex items-start gap-3">
            {last.preview && (
              <img
                src={`data:image/png;base64,${last.preview}`}
                alt="captured region"
                className="h-16 w-16 rounded border border-zinc-700 object-contain"
              />
            )}
            <div className="min-w-0 flex-1">
              <p className="text-xs uppercase tracking-wide text-emerald-400">
                {last.format}
              </p>
              <p className="truncate font-mono text-sm text-zinc-200">
                {last.content}
              </p>
            </div>
            <button
              onClick={() => copy(last.content)}
              className="rounded-lg bg-zinc-800 px-3 py-1.5 text-xs text-zinc-200 transition-colors hover:bg-zinc-700"
            >
              Copy
            </button>
          </div>
        </section>
      )}

      <div className="flex-1 overflow-y-auto">
        <div className="sticky top-0 border-b border-zinc-800 bg-zinc-950 p-3">
          <input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search history…"
            className="w-full rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-100 placeholder-zinc-500 outline-none focus:border-emerald-600"
          />
        </div>

        {filtered.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-24 text-center">
            <p className="text-sm text-zinc-500">
              {history.length === 0
                ? "No scans yet. Press the shortcut, click “Scan Region”, or paste an image."
                : "No results match your search."}
            </p>
          </div>
        ) : (
          <ul className="divide-y divide-zinc-800">
            {filtered.map((h) => (
              <li key={h.id} className="group flex items-start gap-3 px-5 py-3">
                {h.preview ? (
                  <img
                    src={`data:image/png;base64,${h.preview}`}
                    alt="captured region"
                    className="mt-0.5 h-12 w-12 flex-none rounded border border-zinc-700 object-contain"
                  />
                ) : (
                  <div className="mt-0.5 h-12 w-12 flex-none rounded border border-zinc-800" />
                )}
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <span
                      className={`rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide ${
                        h.kind === "code"
                          ? "bg-sky-900/50 text-sky-300"
                          : "bg-violet-900/50 text-violet-300"
                      }`}
                    >
                      {h.kind}
                    </span>
                    <span className="text-[11px] text-zinc-500">{h.format}</span>
                    <span className="text-[11px] text-zinc-600">
                      {formatTimestamp(h.created_at)}
                    </span>
                  </div>
                  <p className="mt-1 break-all font-mono text-sm text-zinc-200">
                    {h.content}
                  </p>
                </div>
                <div className="flex flex-none items-center gap-1">
                  <button
                    onClick={() => copy(h.content)}
                    className="rounded-lg bg-zinc-800 px-3 py-1.5 text-xs text-zinc-200 opacity-0 transition-opacity hover:bg-zinc-700 group-hover:opacity-100"
                  >
                    Copy
                  </button>
                  <button
                    onClick={() => removeEntry(h.id)}
                    className="rounded-lg px-2.5 py-1.5 text-xs text-zinc-500 opacity-0 transition-opacity hover:bg-red-900/40 hover:text-red-300 group-hover:opacity-100"
                    title="Delete entry"
                  >
                    ✕
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </main>
  );
}