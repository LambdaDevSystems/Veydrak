import { describe, expect, it } from 'vitest';
import { autoRange, sparklinePoints } from './sparkline';

describe('sparklinePoints', () => {
  it('maps min to the bottom and max to the top, left-aligned', () => {
    expect(sparklinePoints([0, 10], 0, 10, 2)).toBe('0.0,30.0 100.0,0.0');
  });
  it('clamps out-of-range values', () => {
    expect(sparklinePoints([-5, 50], 0, 10, 2)).toBe('0.0,30.0 100.0,0.0');
  });
  it('handles empty and single-slot input', () => {
    expect(sparklinePoints([], 0, 1)).toBe('');
    expect(sparklinePoints([1], 0, 1, 1)).toBe('0.0,0.0');
  });
});

describe('autoRange', () => {
  it('pads a varying series and widens a flat one', () => {
    expect(autoRange([10, 20])).toEqual({ min: 9, max: 21 });
    expect(autoRange([5, 5])).toEqual({ min: 4, max: 6 });
    expect(autoRange([])).toEqual({ min: 0, max: 1 });
  });
});
