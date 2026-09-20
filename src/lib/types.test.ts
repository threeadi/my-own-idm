import { describe, it, expect } from 'vitest';
import { formatBytes, formatSpeed, formatEta, getPercent, unitToBps, bpsToUnit, formatDisplayVersion } from './types';

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

describe('unitToBps', () => {
  it('converts KB/s to bps correctly', () => {
    expect(unitToBps(500, 'KB/s')).toBe(512000);
    expect(unitToBps(1024, 'KB/s')).toBe(1048576);
  });

  it('converts MB/s to bps correctly', () => {
    expect(unitToBps(1, 'MB/s')).toBe(1048576);
    expect(unitToBps(2.5, 'MB/s')).toBe(2621440);
  });

  it('handles 0 or negative or NaN safely', () => {
    expect(unitToBps(0, 'KB/s')).toBe(0);
    expect(unitToBps(-10, 'MB/s')).toBe(0);
    expect(unitToBps(NaN, 'MB/s')).toBe(0);
  });
});

describe('bpsToUnit', () => {
  it('handles null, undefined, and zero', () => {
    expect(bpsToUnit(null)).toEqual({ value: 0, unit: 'KB/s' });
    expect(bpsToUnit(undefined)).toEqual({ value: 0, unit: 'KB/s' });
    expect(bpsToUnit(0)).toEqual({ value: 0, unit: 'KB/s' });
  });

  it('converts small bps to KB/s', () => {
    expect(bpsToUnit(512000)).toEqual({ value: 500, unit: 'KB/s' });
    expect(bpsToUnit(256000)).toEqual({ value: 250, unit: 'KB/s' });
  });

  it('converts large bps to MB/s', () => {
    expect(bpsToUnit(1048576)).toEqual({ value: 1, unit: 'MB/s' });
    expect(bpsToUnit(2097152)).toEqual({ value: 2, unit: 'MB/s' });
    expect(bpsToUnit(2621440)).toEqual({ value: 2.5, unit: 'MB/s' });
  });
});

describe('formatDisplayVersion', () => {
  it('formats clean versions and prefixes', () => {
    expect(formatDisplayVersion('0.1.0-dev')).toBe('v0.1.0-dev');
    expect(formatDisplayVersion('v0.1.0-dev')).toBe('v0.1.0-dev');
    expect(formatDisplayVersion('1.0.0-beta', 'ver ')).toBe('ver 1.0.0-beta');
  });

  it('handles null, undefined and empty strings', () => {
    expect(formatDisplayVersion(null)).toBe('v0.1.0-dev');
    expect(formatDisplayVersion(undefined)).toBe('v0.1.0-dev');
    expect(formatDisplayVersion('')).toBe('v0.1.0-dev');
    expect(formatDisplayVersion('   ')).toBe('v0.1.0-dev');
  });
});


