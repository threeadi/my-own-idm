export type DownloadCategory =
  | 'all'
  | 'compressed'
  | 'programs'
  | 'video'
  | 'audio'
  | 'documents'
  | 'general';

export type TaskStatus =
  | 'queued'
  | 'probing'
  | 'downloading'
  | 'paused'
  | 'completed'
  | { failed: string };

export interface Segment {
  index: number;
  start_byte: number;
  end_byte: number;
  downloaded_bytes: number;
  is_finished: boolean;
}

export interface DownloadTask {
  id: string;
  url: string;
  filename: string;
  save_dir: string;
  file_path: string;
  total_bytes: number | null;
  downloaded_bytes: number;
  category: DownloadCategory;
  status: TaskStatus;
  connections: number;
  supports_range: boolean;
  is_hls: boolean;
  created_at: string;
  completed_at: string | null;
  error_message: string | null;
  segments: Segment[];
  referer?: string | null;
  speed_limit_bps?: number | null;
  // Computed in frontend
  speed_bps?: number;
  eta_seconds?: number | null;
}

export type SpeedLimitUnit = 'KB/s' | 'MB/s';

export type SortCriterion = 'date' | 'size' | 'name' | 'progress' | 'speed' | 'status';
export type SortOrder = 'asc' | 'desc';

export interface GlobalSpeedLimitConfig {
  enabled: boolean;
  limit_bps: number | null;
}

export interface SpeedMetrics {
  task_id: string;
  speed_bps: number;
  eta_seconds: number | null;
  downloaded_bytes: number;
  total_bytes: number | null;
  percent: number;
  status: TaskStatus;
  segments: Segment[];
}

export interface ProbeResult {
  url: string;
  filename: string;
  total_bytes: number | null;
  formatted_size: string;
  supports_range: boolean;
  category: DownloadCategory;
  suggested_dir: string;
  is_hls: boolean;
}

export interface DuplicateCheckResult {
  is_duplicate: boolean;
  status: TaskStatus | null;
  task_id: string | null;
  filename: string | null;
  file_path: string | null;
  file_exists_on_disk: boolean;
  downloaded_bytes: number;
  total_bytes: number | null;
  percent: number;
  completed_at: string | null;
  suggested_new_filename: string | null;
}

export function formatBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined) return 'Unknown size';
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

export function formatSpeed(bps: number | undefined): string {
  if (!bps || bps === 0) return '0 B/s';
  return formatBytes(bps) + '/s';
}

export function formatEta(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined) return '--:--';
  if (seconds < 60) return `${Math.floor(seconds)}s`;
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  if (m < 60) return `${m}m ${s}s`;
  const h = Math.floor(m / 60);
  const remM = m % 60;
  return `${h}h ${remM}m`;
}

export function getPercent(task: DownloadTask): number {
  if (task.status === 'completed') return 100.0;
  if (!task.total_bytes || task.total_bytes <= 0) return 0.0;
  return Math.min(100.0, (task.downloaded_bytes / task.total_bytes) * 100);
}

export function unitToBps(value: number, unit: SpeedLimitUnit): number {
  if (value <= 0 || isNaN(value)) return 0;
  if (unit === 'MB/s') {
    return Math.round(value * 1024 * 1024);
  }
  return Math.round(value * 1024);
}

export function bpsToUnit(bps: number | null | undefined): { value: number; unit: SpeedLimitUnit } {
  if (!bps || bps <= 0) return { value: 0, unit: 'KB/s' };
  if (bps >= 1024 * 1024) {
    const mb = parseFloat((bps / (1024 * 1024)).toFixed(2));
    if (bps % (1024 * 1024) === 0 || mb >= 1) {
      return { value: mb, unit: 'MB/s' };
    }
  }
  return { value: Math.round(bps / 1024), unit: 'KB/s' };
}

export function formatDisplayVersion(version?: string | null, prefix = 'v'): string {
  if (!version || typeof version !== 'string' || !version.trim()) {
    return `${prefix}0.1.0-dev`;
  }
  const clean = version.trim().replace(/^v/i, '');
  return `${prefix}${clean}`;
}

export type SupportedLanguage = 'id' | 'en';

export interface AppSettings {
  language: SupportedLanguage;
  autoStartWindows: boolean;
  mediaPanelOverlay: boolean;
  clipboardAutoCapture: boolean;
  notifyOnComplete: boolean;
  connectionType: string;
  defaultConnections: number;
  tcpWindowAutoTuning: boolean;
  browserChrome: boolean;
  browserEdge: boolean;
  browserFirefox: boolean;
  browserBrave: boolean;
  defaultDownloadDir: string;
  categorySubfolders: boolean;
  tempDir: string;
  autoCaptureExtensions: string;
  excludedSites: string;
  connectionTimeoutSec: number;
  maxRetries: number;
}

export const DEFAULT_APP_SETTINGS: AppSettings = {
  language: 'id',
  autoStartWindows: true,
  mediaPanelOverlay: true,
  clipboardAutoCapture: true,
  notifyOnComplete: true,
  connectionType: 'broadband',
  defaultConnections: 16,
  tcpWindowAutoTuning: true,
  browserChrome: true,
  browserEdge: true,
  browserFirefox: true,
  browserBrave: true,
  defaultDownloadDir: '',
  categorySubfolders: true,
  tempDir: '',
  autoCaptureExtensions: '3GP 7Z AAC ACE AIF APK ARJ ASF AVI BIN BZ2 EXE GZ GZIP IMG ISO LZH M4A M4V MKV MOV MP3 MP4 MPA MPE MPEG MPG MSI MSU OGG OGV PDF PLJ PPS PPT PPTX QT R0* R1* RA RAR RM RMVB SEA SIT SITX TAR TIF TIFF TS WAV WMA WMV Z ZIP',
  excludedSites: '',
  connectionTimeoutSec: 60,
  maxRetries: 5,
};

export function matchesDownloadExtension(url: string, extensionsStr?: string): boolean {
  if (!url || typeof url !== 'string') return false;
  const cleanUrl = url.trim().toLowerCase();
  if (!cleanUrl.startsWith('http://') && !cleanUrl.startsWith('https://')) return false;

  // Always consider streaming media manifests or direct video/audio urls as download matches
  if (cleanUrl.includes('.m3u8') || cleanUrl.includes('.mpd') || cleanUrl.includes('youtube.com/watch') || cleanUrl.includes('youtu.be/')) {
    return true;
  }

  const rawPath = cleanUrl.split('?')[0].split('#')[0];
  const lastDotIndex = rawPath.lastIndexOf('.');
  if (lastDotIndex === -1) return false;
  const ext = rawPath.substring(lastDotIndex + 1);
  if (!ext || ext.length > 8) return false;

  const exts = (extensionsStr || DEFAULT_APP_SETTINGS.autoCaptureExtensions)
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean);

  return exts.some((e) => {
    if (e.endsWith('*')) {
      const prefix = e.replace(/\*+$/, '');
      return ext.startsWith(prefix);
    }
    return ext === e;
  });
}


