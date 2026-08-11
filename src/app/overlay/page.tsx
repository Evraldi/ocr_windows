"use client";

import { useEffect, useRef, useState } from "react";
import { api } from "@/lib/api";

interface Point {
  x: number;
  y: number;
}

export default function Overlay() {
  const [start, setStart] = useState<Point | null>(null);
  const [current, setCurrent] = useState<Point | null>(null);
  const [finishing, setFinishing] = useState(false);
  const dragging = useRef(false);

  const rect = (() => {
    if (!start || !current) return null;
    const x = Math.min(start.x, current.x);
    const y = Math.min(start.y, current.y);
    const width = Math.abs(current.x - start.x);
    const height = Math.abs(current.y - start.y);
    return { x, y, width, height };
  })();

  const onMouseDown = (e: React.MouseEvent) => {
    if (finishing) return;
    dragging.current = true;
    setStart({ x: e.clientX, y: e.clientY });
    setCurrent({ x: e.clientX, y: e.clientY });
  };

  const onMouseMove = (e: React.MouseEvent) => {
    if (!dragging.current) return;
    setCurrent({ x: e.clientX, y: e.clientY });
  };

  const onMouseUp = () => {
    if (!dragging.current || !start || !current) return;
    dragging.current = false;
    const x = Math.min(start.x, current.x);
    const y = Math.min(start.y, current.y);
    const width = Math.abs(current.x - start.x);
    const height = Math.abs(current.y - start.y);
    if (width > 3 && height > 3) {
      setFinishing(true);
      api
        .finishScan({ x, y, width, height }, window.devicePixelRatio)
        .catch(() => api.cancelScan())
        .finally(() => {
          setFinishing(false);
          setStart(null);
          setCurrent(null);
        });
    } else {
      api.cancelScan().catch(() => undefined);
      setStart(null);
      setCurrent(null);
    }
  };

  useEffect(() => {
    document.documentElement.style.background = "transparent";
    document.body.style.background = "transparent";
    return () => {
      document.documentElement.style.background = "";
      document.body.style.background = "";
    };
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        api.cancelScan().catch(() => undefined);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <div
      className="fixed inset-0 cursor-crosshair select-none overflow-hidden"
      onMouseDown={onMouseDown}
      onMouseMove={onMouseMove}
      onMouseUp={onMouseUp}
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
