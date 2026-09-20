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
  // Computed in frontend
  speed_bps?: number;
  eta_seconds?: number | null;
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

