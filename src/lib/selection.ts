export interface Point {
  x: number;
  y: number;
}

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Batasi kursor ke viewport overlay supaya region tidak keluar layar. */
export function clampPoint(x: number, y: number, maxX: number, maxY: number): Point {
  return {
    x: Math.max(0, Math.min(x, maxX)),
    y: Math.max(0, Math.min(y, maxY)),
  };
}

/** Bentuk rectangle dari dua titik, arah drag boleh terbalik. */
export function rectBetween(a: Point | null, b: Point | null): Rect | null {
  if (!a || !b) return null;
  return {
    x: Math.min(a.x, b.x),
    y: Math.min(a.y, b.y),
    width: Math.abs(b.x - a.x),
    height: Math.abs(b.y - a.y),
  };
}
