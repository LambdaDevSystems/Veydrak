import { HISTORY_LEN } from '../stores/sysmon';

const W = 100;
const H = 30;

/** SVG polyline points for `values` in a 100x30 box, left-aligned over `len` slots, scaled to [min, max]. */
export function sparklinePoints(values: number[], min: number, max: number, len = HISTORY_LEN): string {
  const span = max - min || 1;
  return values
    .map((v, i) => {
      const clamped = Math.min(Math.max(v, min), max);
      const x = (i / Math.max(len - 1, 1)) * W;
      const y = H - ((clamped - min) / span) * H;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(' ');
}

/** Autoscale range with a little headroom; a flat series gets a small symmetric range. */
export function autoRange(values: number[]): { min: number; max: number } {
  if (values.length === 0) return { min: 0, max: 1 };
  const lo = Math.min(...values);
  const hi = Math.max(...values);
  if (lo === hi) return { min: lo - 1, max: hi + 1 };
  const pad = (hi - lo) * 0.1;
  return { min: lo - pad, max: hi + pad };
}
