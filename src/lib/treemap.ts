// Squarified treemap (Bruls, Huizing & van Wijk): lays out values as rectangles whose areas are
// proportional to the values, keeping aspect ratios close to 1 so small items stay clickable.

export type Rect = { x: number; y: number; w: number; h: number };
export type Tile<T> = Rect & { item: T; value: number };

function worst(row: number[], side: number, scale: number): number {
  const sum = row.reduce((a, b) => a + b, 0) * scale;
  if (sum === 0) return Infinity;
  const max = Math.max(...row) * scale;
  const min = Math.min(...row) * scale;
  const s2 = side * side;
  return Math.max((s2 * max) / (sum * sum), (sum * sum) / (s2 * min));
}

/** Lays out `items` (any order; sorted largest first internally) inside `rect`. Zero values are dropped. */
export function squarify<T>(items: { value: number; item: T }[], rect: Rect): Tile<T>[] {
  const sorted = items.filter((i) => i.value > 0).sort((a, b) => b.value - a.value);
  const total = sorted.reduce((s, i) => s + i.value, 0);
  if (!total || rect.w <= 0 || rect.h <= 0) return [];
  const scale = (rect.w * rect.h) / total; // area per unit of value
  const out: Tile<T>[] = [];
  let free = { ...rect };
  let row: typeof sorted = [];

  const layoutRow = (r: typeof sorted) => {
    const area = r.reduce((s, i) => s + i.value, 0) * scale;
    const horizontal = free.w >= free.h; // lay the row along the shorter side
    if (horizontal) {
      const width = area / free.h;
      let y = free.y;
      for (const i of r) {
        const h = (i.value * scale) / width;
        out.push({ x: free.x, y, w: width, h, item: i.item, value: i.value });
        y += h;
      }
      free = { x: free.x + width, y: free.y, w: free.w - width, h: free.h };
    } else {
      const height = area / free.w;
      let x = free.x;
      for (const i of r) {
        const w = (i.value * scale) / height;
        out.push({ x, y: free.y, w, h: height, item: i.item, value: i.value });
        x += w;
      }
      free = { x: free.x, y: free.y + height, w: free.w, h: free.h - height };
    }
  };

  for (const item of sorted) {
    const side = Math.min(free.w, free.h);
    const values = row.map((r) => r.value);
    if (row.length === 0 || worst([...values, item.value], side, scale) <= worst(values, side, scale)) {
      row.push(item);
    } else {
      layoutRow(row);
      row = [item];
    }
  }
  if (row.length) layoutRow(row);
  return out;
}
