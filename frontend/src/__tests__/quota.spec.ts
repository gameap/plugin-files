import { describe, expect, it } from 'vitest';
import { formatSize, parseSize } from '@/composables/useQuotaInput';

describe('parseSize', () => {
  it('treats empty and zero as unlimited', () => {
    expect(parseSize('')).toBe(0);
    expect(parseSize('  ')).toBe(0);
    expect(parseSize('0')).toBe(0);
  });

  it('parses sizes with units (1024-based)', () => {
    expect(parseSize('512')).toBe(512);
    expect(parseSize('500K')).toBe(500 * 1024);
    expect(parseSize('10MB')).toBe(10 * 1024 ** 2);
    expect(parseSize('1.5 GB')).toBe(Math.round(1.5 * 1024 ** 3));
    expect(parseSize('2tb')).toBe(2 * 1024 ** 4);
  });

  it('rejects invalid input', () => {
    expect(parseSize('abc')).toBeNull();
    expect(parseSize('-5MB')).toBeNull();
    expect(parseSize('10XB')).toBeNull();
  });
});

describe('formatSize', () => {
  it('renders unlimited as empty string', () => {
    expect(formatSize(0)).toBe('');
  });

  it('formats whole and fractional sizes', () => {
    expect(formatSize(512)).toBe('512 B');
    expect(formatSize(10 * 1024 ** 3)).toBe('10 GB');
    expect(formatSize(Math.round(1.5 * 1024 ** 3))).toBe('1.5 GB');
  });

  it('round-trips through parseSize', () => {
    for (const bytes of [1024, 5 * 1024 ** 2, 10 * 1024 ** 3]) {
      expect(parseSize(formatSize(bytes))).toBe(bytes);
    }
  });
});
