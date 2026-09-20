import { describe, it, expect } from 'vitest';
import { formatBytes, formatSpeed, formatEta, getPercent } from './types';

describe('formatBytes', () => {
  it('handles null and undefined', () => {
    expect(formatBytes(null)).toBe('Unknown size');
    expect(formatBytes(undefined)).toBe('Unknown size');
  });

  it('formats zero bytes', () => {
    expect(formatBytes(0)).toBe('0 B');
  });

  it('formats byte boundaries correctly', () => {
    expect(formatBytes(500)).toBe('500 B');
    expect(formatBytes(1024)).toBe('1 KB');
    expect(formatBytes(1024 * 1024)).toBe('1 MB');
    expect(formatBytes(1024 * 1024 * 10.5)).toBe('10.5 MB');
    expect(formatBytes(1024 * 1024 * 1024 * 2.5)).toBe('2.5 GB');
  });
});

describe('formatSpeed', () => {
  it('handles zero or undefined speed', () => {
    expect(formatSpeed(0)).toBe('0 B/s');
    expect(formatSpeed(undefined)).toBe('0 B/s');
  });

  it('formats active speeds', () => {
    expect(formatSpeed(1024)).toBe('1 KB/s');
    expect(formatSpeed(1048576 * 5)).toBe('5 MB/s');
  });
});

describe('formatEta', () => {
  it('handles null and undefined eta', () => {
    expect(formatEta(null)).toBe('--:--');
    expect(formatEta(undefined)).toBe('--:--');
  });

  it('formats seconds (<60s)', () => {
    expect(formatEta(30)).toBe('30s');
    expect(formatEta(59)).toBe('59s');
  });

  it('formats minutes and seconds (1m - 59m)', () => {
    expect(formatEta(65)).toBe('1m 5s');
    expect(formatEta(180)).toBe('3m 0s');
  });

  it('formats hours and minutes (>= 1h)', () => {
    expect(formatEta(3600)).toBe('1h 0m');
    expect(formatEta(3665)).toBe('1h 1m');
    expect(formatEta(7300)).toBe('2h 1m');
  });
});

describe('getPercent', () => {
  it('calculates percentage accurately and respects completed state', () => {
    expect(
      getPercent({ status: 'completed', downloaded_bytes: 100, total_bytes: 200 } as any)
    ).toBe(100.0);
    expect(
      getPercent({ status: 'downloading', downloaded_bytes: 50, total_bytes: 200 } as any)
    ).toBe(25.0);
    expect(
      getPercent({ status: 'downloading', downloaded_bytes: 0, total_bytes: null } as any)
    ).toBe(0.0);
    expect(
      getPercent({ status: 'downloading', downloaded_bytes: 0, total_bytes: 0 } as any)
    ).toBe(0.0);
  });
});

