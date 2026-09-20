import { invoke, isTauri as coreIsTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { DownloadCategory, DownloadTask, SpeedMetrics } from './types';

export function isTauri(): boolean {
  if (typeof window === 'undefined') return false;
  return coreIsTauri() || '__TAURI_INTERNALS__' in window || 'isTauri' in window;
}

export class IdmStore {
  tasks = $state<DownloadTask[]>([]);
  selectedTaskId = $state<string | null>(null);
  activeCategory = $state<string>('all');
  searchQuery = $state<string>('');
  isAddModalOpen = $state<boolean>(false);
  isSettingsModalOpen = $state<boolean>(false);
  isProgressModalOpen = $state<boolean>(false);
  progressModalTaskId = $state<string | null>(null);
  viewMode = $state<'cards' | 'table'>('cards');
  sortBy = $state<'date' | 'size' | 'name'>('date');
  speedLimiterEnabled = $state<boolean>(false);
  initialAddUrl = $state<string>('');
  initialFilename = $state<string>('');
  initialHeaders = $state<Record<string, string> | null>(null);

  isOutcomeModalOpen = $state<boolean>(false);
  outcomeTaskId = $state<string | null>(null);
  outcomeType = $state<'completed' | 'failed'>('completed');
  outcomeErrorMessage = $state<string | null>(null);

  isPropertiesModalOpen = $state<boolean>(false);
  propertiesTaskId = $state<string | null>(null);

  selectedTask = $derived<DownloadTask | null>(
    this.tasks.find((t) => t.id === this.selectedTaskId) || null
  );

  progressModalTask = $derived<DownloadTask | null>(
    this.tasks.find((t) => t.id === (this.progressModalTaskId || this.selectedTaskId)) || null
  );

  outcomeTask = $derived<DownloadTask | null>(
    this.tasks.find((t) => t.id === this.outcomeTaskId) || null
  );

  propertiesTask = $derived<DownloadTask | null>(
    this.tasks.find((t) => t.id === this.propertiesTaskId) || null
  );


  filteredTasks = $derived.by<DownloadTask[]>(() => {
    return this.tasks.filter((task) => {
      // Category / Status Filter
      if (this.activeCategory === 'downloading') {
        if (task.status !== 'downloading') return false;
      } else if (this.activeCategory === 'completed') {
        if (task.status !== 'completed') return false;
      } else if (this.activeCategory === 'paused') {
        if (task.status !== 'paused') return false;
      } else if (this.activeCategory !== 'all') {
        if (task.category !== this.activeCategory) return false;
      }

      // Search Query Filter
      if (this.searchQuery.trim()) {
        const q = this.searchQuery.toLowerCase();
        return (
          task.filename.toLowerCase().includes(q) ||
          task.url.toLowerCase().includes(q)
        );
      }
      return true;
    });
  });

  categoryCounts = $derived.by<Record<string, number>>(() => {
    const counts: Record<string, number> = {
      all: this.tasks.length,
      downloading: 0,
      completed: 0,
      paused: 0,
      video: 0,
      audio: 0,
      compressed: 0,
      programs: 0,
      documents: 0,
      general: 0,
    };

    for (const t of this.tasks) {
      if (t.status === 'downloading') counts.downloading++;
      if (t.status === 'completed') counts.completed++;
      if (t.status === 'paused') counts.paused++;
      if (counts[t.category] !== undefined) {
        counts[t.category]++;
      }
    }
    return counts;
  });

  totalSpeedBps = $derived.by<number>(() => {
    let speed = 0;
    for (const t of this.tasks) {
      if (t.status === 'downloading' && t.speed_bps) {
        speed += t.speed_bps;
      }
    }
    return speed;
  });

  async init() {
    if (!isTauri()) {
      console.info('Running in browser preview mode (Tauri IPC inactive).');
      if (this.tasks.length === 0) {
        this.tasks = [
          {
            id: 'mock-1',
            url: "https://video.twimg.com/amplify_video/2079553321615118336/pl/avc1/1920x1080/m3u8_stream_v7.mp4",
            filename: "Lust Hunter di X - 'Dispatch - NTR final project'.mp4",
            save_dir: "D:\\IDM_Downloads\\Videos",
            file_path: "D:\\IDM_Downloads\\Videos\\Lust Hunter di X - 'Dispatch - NTR final project'.mp4",
            total_bytes: 46497792,
            downloaded_bytes: 46497792,
            category: "video",
            status: "completed",
            connections: 16,
            supports_range: true,
            is_hls: true,
            created_at: "2026-09-20 14:32:00",
            completed_at: "2026-09-20 14:32:11",
            speed_bps: 0,
            eta_seconds: 0,
          },
          {
            id: 'mock-2',
            url: "https://releases.ubuntu.com/22.04/ubuntu-22.04.4-desktop-amd64.iso",
            filename: "ubuntu-22.04.4-desktop-amd64.iso",
            save_dir: "D:\\IDM_Downloads\\Programs",
            file_path: "D:\\IDM_Downloads\\Programs\\ubuntu-22.04.4-desktop-amd64.iso",
            total_bytes: 4975600000,
            downloaded_bytes: 3680000000,
            category: "programs",
            status: "downloading",
            connections: 16,
            supports_range: true,
            is_hls: false,
            created_at: "2026-09-20 14:40:00",
            speed_bps: 12500000,
            eta_seconds: 103,
          },
          {
            id: 'mock-3',
            url: "https://example.com/interrupted-archive.zip",
            filename: "interrupted-archive.zip",
            save_dir: "D:\\IDM_Downloads\\Compressed",
            file_path: "D:\\IDM_Downloads\\Compressed\\interrupted-archive.zip",
            total_bytes: 524288000,
            downloaded_bytes: 388000000,
            category: "compressed",
            status: { failed: "HTTP 504 Gateway Timeout" },
            connections: 8,
            supports_range: true,
            is_hls: false,
            created_at: "2026-09-20 14:10:00",
            error_message: "HTTP 504 Gateway Timeout / Sambungan Ditolak oleh Host Server",
            speed_bps: 0,
            eta_seconds: 0,
          },
        ];
        this.selectedTaskId = 'mock-1';
      }
      return;
    }
    await this.refreshTasks();
    await this.setupEventListeners();
  }

  async refreshTasks() {
    if (!isTauri()) return;
    try {
      const data = await invoke<DownloadTask[]>('get_all_tasks');
      this.tasks = data;
      if (this.tasks.length > 0 && !this.selectedTaskId) {
        this.selectedTaskId = this.tasks[0].id;
      }
    } catch (e) {
      console.error('Failed to load tasks:', e);
    }
  }

  async setupEventListeners() {
    // 1. Progress event
    await listen<SpeedMetrics>('download-progress', (event) => {
      const m = event.payload;
      const idx = this.tasks.findIndex((t) => t.id === m.task_id);
      if (idx !== -1) {
        this.tasks[idx].downloaded_bytes = m.downloaded_bytes;
        if (m.total_bytes) {
          this.tasks[idx].total_bytes = m.total_bytes;
        }
        this.tasks[idx].speed_bps = m.speed_bps;
        this.tasks[idx].eta_seconds = m.eta_seconds;
        this.tasks[idx].status = m.status;
        this.tasks[idx].segments = m.segments;
        if (m.status === 'completed') {
          this.tasks[idx].downloaded_bytes = this.tasks[idx].total_bytes ?? m.downloaded_bytes;
        }
      }
    });

    // 2. Completed event
    await listen<string>('download-completed', (event) => {
      const id = event.payload;
      const idx = this.tasks.findIndex((t) => t.id === id);
      if (idx !== -1) {
        this.tasks[idx].status = 'completed';
        this.tasks[idx].speed_bps = 0;
        this.tasks[idx].eta_seconds = 0;
        if (this.tasks[idx].total_bytes) {
          this.tasks[idx].downloaded_bytes = this.tasks[idx].total_bytes;
        }
      }
      if (this.isProgressModalOpen && (this.progressModalTaskId === id || !this.progressModalTaskId)) {
        this.closeProgressModal();
      }
      this.openOutcomeModal(id, 'completed');
    });

    // 3. Paused event
    await listen<string>('download-paused', (event) => {
      const id = event.payload;
      const idx = this.tasks.findIndex((t) => t.id === id);
      if (idx !== -1) {
        this.tasks[idx].status = 'paused';
        this.tasks[idx].speed_bps = 0;
      }
    });

    // 4. Failed event
    await listen<string>('download-failed', (event) => {
      const id = event.payload;
      const idx = this.tasks.findIndex((t) => t.id === id);
      let errMsg = 'Gagal mengunduh berkas atau koneksi terputus.';
      if (idx !== -1) {
        if (typeof this.tasks[idx].status === 'object' && 'failed' in this.tasks[idx].status) {
          errMsg = (this.tasks[idx].status as any).failed;
        } else {
          this.tasks[idx].status = { failed: errMsg };
        }
        this.tasks[idx].speed_bps = 0;
      }
      if (this.isProgressModalOpen && (this.progressModalTaskId === id || !this.progressModalTaskId)) {
        this.closeProgressModal();
      }
      this.openOutcomeModal(id, 'failed', errMsg);
    });

    // 5. Browser Extension download request
    await listen<any>('browser-download-requested', (event) => {
      console.log('browser-download-requested received:', event.payload);
      const p = event.payload;
      if (p && p.url) {
        this.openAddModal(p.url, p.filename, p.headers);
      }
    });
  }

  async pauseTask(taskId: string) {
    try {
      await invoke('pause_download', { taskId });
      const idx = this.tasks.findIndex((t) => t.id === taskId);
      if (idx !== -1) {
        this.tasks[idx].status = 'paused';
        this.tasks[idx].speed_bps = 0;
      }
    } catch (e) {
      console.error('Pause failed:', e);
    }
  }

  async resumeTask(taskId: string) {
    try {
      await invoke('resume_download', { taskId });
      const idx = this.tasks.findIndex((t) => t.id === taskId);
      if (idx !== -1) {
        this.tasks[idx].status = 'downloading';
      }
    } catch (e) {
      console.error('Resume failed:', e);
    }
  }

  async cancelTask(taskId: string, deleteFile: boolean = false) {
    try {
      await invoke('cancel_download', { taskId, deleteFile });
      this.tasks = this.tasks.filter((t) => t.id !== taskId);
      if (this.selectedTaskId === taskId) {
        this.selectedTaskId = this.tasks.length > 0 ? this.tasks[0].id : null;
      }
    } catch (e) {
      console.error('Cancel failed:', e);
    }
  }

  async openFile(filePath: string) {
    try {
      await invoke('open_file', { path: filePath });
    } catch (e) {
      console.error('Open file failed:', e);
    }
  }

  async openFolder(filePath: string) {
    try {
      await invoke('open_file_in_folder', { path: filePath });
    } catch (e) {
      console.error('Open folder failed:', e);
    }
  }

  openAddModal(url: string = '', filename: string = '', headers: Record<string, string> | null = null) {
    this.initialAddUrl = url;
    this.initialFilename = filename || '';
    this.initialHeaders = headers || null;
    this.isAddModalOpen = true;
  }

  openProgressModal(taskId?: string) {
    if (taskId) {
      this.progressModalTaskId = taskId;
      this.selectedTaskId = taskId;
    }
    this.isProgressModalOpen = true;
  }

  closeProgressModal() {
    this.isProgressModalOpen = false;
    this.progressModalTaskId = null;
  }

  openOutcomeModal(taskId: string, type: 'completed' | 'failed' = 'completed', errorMsg?: string | null) {
    this.outcomeTaskId = taskId;
    this.outcomeType = type;
    this.outcomeErrorMessage = errorMsg || null;
    this.isOutcomeModalOpen = true;
  }

  closeOutcomeModal() {
    this.isOutcomeModalOpen = false;
    this.outcomeTaskId = null;
    this.outcomeErrorMessage = null;
  }

  openPropertiesModal(taskId: string) {
    this.propertiesTaskId = taskId;
    this.isPropertiesModalOpen = true;
  }

  closePropertiesModal() {
    this.isPropertiesModalOpen = false;
    this.propertiesTaskId = null;
  }

  async moveTaskFile(taskId: string, newDir: string) {
    if (!isTauri()) return;
    try {
      const updatedTask = await invoke<DownloadTask>('move_downloaded_file', {
        taskId,
        newDir,
      });
      const idx = this.tasks.findIndex((t) => t.id === taskId);
      if (idx !== -1) {
        this.tasks[idx] = updatedTask;
      }
    } catch (e) {
      console.error('Move file failed:', e);
      throw e;
    }
  }

  async getRecentLogs(maxLines: number = 100): Promise<string[]> {
    if (!isTauri()) return ['[Preview Mode] No active Tauri logger.'];
    try {
      return await invoke<string[]>('get_recent_logs', { maxLines });
    } catch (e) {
      console.error('Failed to get logs:', e);
      return [];
    }
  }

  async resumeAll() {
    const paused = this.tasks.filter((t) => t.status === 'paused' || (typeof t.status === 'object' && 'failed' in t.status));
    for (const t of paused) {
      await this.resumeTask(t.id);
    }
  }

  async pauseAll() {
    const active = this.tasks.filter((t) => t.status === 'downloading');
    for (const t of active) {
      await this.pauseTask(t.id);
    }
  }

  async clearCompleted() {
    const completed = this.tasks.filter((t) => t.status === 'completed');
    for (const t of completed) {
      await this.cancelTask(t.id, false);
    }
  }
}

export const store = new IdmStore();

