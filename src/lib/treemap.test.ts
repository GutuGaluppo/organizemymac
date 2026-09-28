import { describe, expect, it } from "vitest";
import { squarify } from "./treemap";

const rect = { x: 0, y: 0, w: 600, h: 400 };

describe("squarify", () => {
  it("areas are proportional to values and fill the rectangle", () => {
    const values = [6, 6, 4, 3, 2, 2, 1];
    const tiles = squarify(values.map((v, i) => ({ value: v, item: i })), rect);
    const total = values.reduce((a, b) => a + b, 0);
    expect(tiles).toHaveLength(values.length);
    let area = 0;
    for (const t of tiles) {
      expect(t.w * t.h).toBeCloseTo((t.value / total) * rect.w * rect.h, 6);
      area += t.w * t.h;
    }
    expect(area).toBeCloseTo(rect.w * rect.h, 6);
  });

  it("tiles stay inside the rectangle and do not overlap", () => {
    const tiles = squarify(Array.from({ length: 40 }, (_, i) => ({ value: (i % 7) + 1 + i * 0.3, item: i })), rect);
    const eps = 1e-6;
    for (const t of tiles) {
      expect(t.x).toBeGreaterThanOrEqual(-eps);
      expect(t.y).toBeGreaterThanOrEqual(-eps);
      expect(t.x + t.w).toBeLessThanOrEqual(rect.w + eps);
      expect(t.y + t.h).toBeLessThanOrEqual(rect.h + eps);
    }
    for (let i = 0; i < tiles.length; i++)
      for (let j = i + 1; j < tiles.length; j++) {
        const a = tiles[i];
        const b = tiles[j];
        const overlapX = Math.min(a.x + a.w, b.x + b.w) - Math.max(a.x, b.x);
        const overlapY = Math.min(a.y + a.h, b.y + b.h) - Math.max(a.y, b.y);
        expect(overlapX > eps && overlapY > eps).toBe(false);
      }
  });

  it("keeps aspect ratios reasonable for equal values", () => {
    const tiles = squarify(Array.from({ length: 24 }, (_, i) => ({ value: 1, item: i })), rect);
    const worst = Math.max(...tiles.map((t) => Math.max(t.w / t.h, t.h / t.w)));
    expect(worst).toBeLessThan(3);
  });

  it("drops zero values and handles empty input", () => {
    expect(squarify([{ value: 0, item: 0 }], rect)).toEqual([]);
    expect(squarify([], rect)).toEqual([]);
  });
});
