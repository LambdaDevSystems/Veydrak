import { describe, expect, it } from 'vitest';
import { formatBytes, sanitizeDeviceName } from './format';

describe('formatBytes', () => {
  it('formats units', () => {
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(2048)).toBe('2.0 KiB');
    expect(formatBytes(-1)).toBe('0 B');
  });
});

describe('sanitizeDeviceName', () => {
  it('strips control and bidi characters', () => {
    expect(sanitizeDeviceName('pr\u001b[31mint‮er')).toBe('pr[31minter');
  });
  it('falls back and truncates', () => {
    expect(sanitizeDeviceName('  \n ')).toBe('unknown device');
    expect(sanitizeDeviceName('a'.repeat(100)).length).toBe(48);
  });
});
