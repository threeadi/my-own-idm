declare const __APP_VERSION__: string | undefined;

export const DEFAULT_APP_VERSION = '0.1.0-dev';

/**
 * Returns the compile-time version injected by Vite from package.json or APP_VERSION env var.
 */
export function getCompileTimeVersion(): string {
  if (typeof __APP_VERSION__ !== 'undefined' && __APP_VERSION__) {
    return __APP_VERSION__;
  }
  return DEFAULT_APP_VERSION;
}

/**
 * Attempts to retrieve the runtime application version from Tauri native app API,
 * falling back gracefully to the compile-time injected version.
 */
export async function fetchRuntimeAppVersion(): Promise<string> {
  try {
    const { getVersion } = await import('@tauri-apps/api/app');
    const v = await getVersion();
    if (v && typeof v === 'string' && v.trim().length > 0) {
      return v.trim();
    }
  } catch {
    // Non-Tauri web environment or testing fallback
  }
  return getCompileTimeVersion();
}
