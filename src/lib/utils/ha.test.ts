import { describe, expect, it } from 'vitest';
import { isUnavailable, mergeEntities, numericState, type HaEntity } from './ha';

const ent = (id: string, state: string, extra: Partial<HaEntity> = {}): HaEntity => ({
  entity_id: id,
  state,
  unit: null,
  friendly_name: null,
  last_updated: null,
  ...extra,
});

describe('numericState', () => {
  it('parses numbers only', () => {
    expect(numericState('24.5')).toBe(24.5);
    expect(numericState('-3')).toBe(-3);
    expect(numericState('on')).toBeNull();
    expect(numericState('')).toBeNull();
    expect(numericState('unavailable')).toBeNull();
    expect(numericState('Infinity')).toBeNull();
  });
  it('flags unavailable states', () => {
    expect(isUnavailable('unavailable')).toBe(true);
    expect(isUnavailable('unknown')).toBe(true);
    expect(isUnavailable('0')).toBe(false);
  });
});

describe('mergeEntities', () => {
  it('accumulates numeric history and skips non-numeric states', () => {
    let rows = mergeEntities({}, [ent('sensor.t', '20', { unit: '°C', friendly_name: 'Temp' })]);
    rows = mergeEntities(rows, [ent('sensor.t', 'unavailable')]);
    rows = mergeEntities(rows, [ent('sensor.t', '21')]);
    expect(rows['sensor.t'].history).toEqual([20, 21]);
    expect(rows['sensor.t'].entity.state).toBe('21');
    // unit and name survive updates that omit them
    expect(rows['sensor.t'].entity.unit).toBe('°C');
    expect(rows['sensor.t'].entity.friendly_name).toBe('Temp');
  });
  it('does not mutate the input map', () => {
    const before = mergeEntities({}, [ent('a.b', '1')]);
    mergeEntities(before, [ent('a.b', '2')]);
    expect(before['a.b'].history).toEqual([1]);
  });
});
