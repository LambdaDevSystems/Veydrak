import { describe, expect, it } from 'vitest';
import { resolvePanels } from './resolve';

const reg = { terminal: 1, sysmon: 2, lan: 3 };

describe('resolvePanels', () => {
  it('keeps known ids in order and drops unknown ones', () => {
    expect(resolvePanels(['lan', 'nope', 'sysmon'], reg).map((p) => p.id)).toEqual(['lan', 'sysmon']);
    expect(resolvePanels([], reg)).toEqual([]);
  });
  it('ignores prototype keys', () => {
    expect(resolvePanels(['toString', '__proto__'], reg)).toEqual([]);
  });
});
