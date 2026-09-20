import { describe, it, expect, vi, beforeEach } from 'vitest';
import { getCompileTimeVersion, fetchRuntimeAppVersion, DEFAULT_APP_VERSION } from './version';

describe('version module', () => {
  beforeEach(() => {
    vi.resetModules();
  });

  it('returns DEFAULT_APP_VERSION if __APP_VERSION__ is not defined', () => {
    expect(DEFAULT_APP_VERSION).toBe('0.1.0-dev');
    const v = getCompileTimeVersion();
    expect(v).toBeDefined();
    expect(typeof v).toBe('string');
  });

  it('fetches runtime version from tauri app api when available', async () => {
    vi.doMock('@tauri-apps/api/app', () => ({
      getVersion: vi.fn().mockResolvedValue('0.2.0-dev.2'),
    }));

    const { fetchRuntimeAppVersion: dynamicFetch } = await import('./version');
    const version = await dynamicFetch();
    expect(version).toBe('0.2.0-dev.2');
  });

  it('falls back to compile time version if getVersion throws', async () => {
    vi.doMock('@tauri-apps/api/app', () => ({
      getVersion: vi.fn().mockRejectedValue(new Error('Tauri not running')),
    }));

    const { fetchRuntimeAppVersion: dynamicFetch } = await import('./version');
    const version = await dynamicFetch();
    expect(version).toBe(getCompileTimeVersion());
  });

  it('falls back to compile time version if getVersion returns empty string', async () => {
    vi.doMock('@tauri-apps/api/app', () => ({
      getVersion: vi.fn().mockResolvedValue('   '),
    }));

    const { fetchRuntimeAppVersion: dynamicFetch } = await import('./version');
    const version = await dynamicFetch();
    expect(version).toBe(getCompileTimeVersion());
  });
});
