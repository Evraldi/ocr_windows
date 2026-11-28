"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "@/lib/api";
import { clampPoint, rectBetween } from "@/lib/selection";
import type { Point, Rect } from "@/lib/selection";

const clamp = (x: number, y: number): Point =>
  clampPoint(x, y, window.innerWidth, window.innerHeight);

export default function Overlay() {
  // Pakai ref untuk koordinat: mouseup bisa terjadi di luar webview, jadi
  // state React bisa stale kalau cuma mengandalkan event handler di elemen.
  const start = useRef<Point | null>(null);
  const current = useRef<Point | null>(null);
  const dragging = useRef(false);
  const finishingRef = useRef(false);
  const [rect, setRect] = useState<Rect | null>(null);
  const [finishing, setFinishing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const reset = useCallback(() => {
    dragging.current = false;
    start.current = null;
    current.current = null;
    setRect(null);
  }, []);

  const onMouseDown = useCallback(
    (e: React.MouseEvent) => {
      if (finishingRef.current || e.button !== 0) return;
      const p = clamp(e.clientX, e.clientY);
      dragging.current = true;
      start.current = p;
      current.current = p;
      setRect(rectBetween(p, p));
    },
    []
  );

  const onMouseUp = useCallback(() => {
    if (!dragging.current || finishingRef.current) return;
    dragging.current = false;

    const selected = rectBetween(start.current, current.current);
    if (!selected || selected.width <= 3 || selected.height <= 3) {
      reset();
      api.cancelScan().catch(() => undefined);
      return;
    }

    finishingRef.current = true;
    setFinishing(true);
    api
      .finishScan(selected, window.devicePixelRatio)
      .catch((err) => {
        api.cancelScan().catch(() => undefined);
        setError(String(err));
      })
      .finally(() => {
        finishingRef.current = false;
        setFinishing(false);
        reset();
      });
  }, [reset]);

  useEffect(() => {
    const onMouseMove = (e: MouseEvent) => {
      if (!dragging.current) return;
      current.current = clamp(e.clientX, e.clientY);
      setRect(rectBetween(start.current, current.current));
    };
    const onBlur = () => {
      if (!dragging.current) return;
      reset();
      api.cancelScan().catch(() => undefined);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      reset();
      api.cancelScan().catch(() => undefined);
    };

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
    window.addEventListener("blur", onBlur);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
      window.removeEventListener("blur", onBlur);
      window.removeEventListener("keydown", onKey);
    };
  }, [onMouseUp, reset]);

  useEffect(() => {
    document.documentElement.style.background = "transparent";
    document.body.style.background = "transparent";
    return () => {
      document.documentElement.style.background = "";
      document.body.style.background = "";
    };
  }, []);

  useEffect(() => {
    const onFocus = () => setError(null);
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, []);

  useEffect(() => {
    if (!error) return;
    const timer = setTimeout(() => setError(null), 4000);
    return () => clearTimeout(timer);
  }, [error]);

  return (
    <div
      className="fixed inset-0 cursor-crosshair select-none overflow-hidden"
      onMouseDown={onMouseDown}
    >
      <div className="absolute inset-0 bg-black/20" />

      {rect && (
        <div
          className="absolute border-2 border-sky-400 shadow-[0_0_0_9999px_rgba(0,0,0,0.35)]"
          style={{
            left: rect.x,
            top: rect.y,
            width: rect.width,
            height: rect.height,
          }}
        >
          <div className="absolute -top-8 left-0 rounded bg-sky-500 px-2 py-0.5 font-mono text-xs text-white">
            {Math.round(rect.width)} × {Math.round(rect.height)}
          </div>
        </div>
      )}

      {!rect && (
        <div className="pointer-events-none absolute left-1/2 top-10 -translate-x-1/2 rounded bg-black/60 px-4 py-2 text-center text-sm text-white">
          Drag to select a region
          <br />
          <span className="text-xs text-zinc-300">Press Esc to cancel</span>
        </div>
      )}

      {error && (
        <div className="pointer-events-none absolute left-1/2 top-24 -translate-x-1/2 rounded bg-red-900/90 px-4 py-2 text-center text-sm text-red-100">
          {error}
        </div>
      )}

      {finishing && (
        <div className="pointer-events-none absolute inset-0 flex items-center justify-center">
          <div className="rounded bg-black/70 px-4 py-2 text-sm text-white">
            Scanning…
          </div>
        </div>
      )}
    </div>
  );
}
