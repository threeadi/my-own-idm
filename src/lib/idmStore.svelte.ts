import { invoke, isTauri as coreIsTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { DownloadCategory, DownloadTask, SpeedMetrics, SpeedLimitUnit, GlobalSpeedLimitConfig, AppSettings, DuplicateCheckResult, SortCriterion, SortOrder, SupportedLanguage } from './types';
import { unitToBps, bpsToUnit, DEFAULT_APP_SETTINGS, matchesDownloadExtension } from './types';
import { getCompileTimeVersion, fetchRuntimeAppVersion } from './version';
import { getTranslation, type TranslationKey } from './i18n';

export function isTauri(): boolean {
  if (typeof window === 'undefined') return false;
  return coreIsTauri() || '__TAURI_INTERNALS__' in window || 'isTauri' in window;
}

export class IdmStore {
  appVersion = $state<string>(getCompileTimeVersion());
  settings = $state<AppSettings>({ ...DEFAULT_APP_SETTINGS });
  lastCheckedClipboard = $state<string>('');
  tasks = $state<DownloadTask[]>([]);
  selectedTaskId = $state<string | null>(null);
  activeCategory = $state<string>('all');
  searchQuery = $state<string>('');
  isAddModalOpen = $state<boolean>(false);
  isSettingsModalOpen = $state<boolean>(false);
  isProgressModalOpen = $state<boolean>(false);
  progressModalTaskId = $state<string | null>(null);
  viewMode = $state<'cards' | 'table'>('cards');
  sortBy = $state<SortCriterion>('date');
  sortOrder = $state<SortOrder>('desc');
  speedLimiterEnabled = $state<boolean>(false);
  globalSpeedLimitValue = $state<number>(1);
  globalSpeedLimitUnit = $state<SpeedLimitUnit>('MB/s');
  initialAddUrl = $state<string>('');
  initialFilename = $state<string>('');
  initialHeaders = $state<Record<string, string> | null>(null);

  globalSpeedLimitBps = $derived<number | null>(
    this.speedLimiterEnabled ? unitToBps(this.globalSpeedLimitValue, this.globalSpeedLimitUnit) : null
  );

  isOutcomeModalOpen = $state<boolean>(false);
  outcomeTaskId = $state<string | null>(null);
  outcomeType = $state<'completed' | 'failed'>('completed');
  outcomeErrorMessage = $state<string | null>(null);

  isPropertiesModalOpen = $state<boolean>(false);
  propertiesTaskId = $state<string | null>(null);

  isRefreshModalOpen = $state<boolean>(false);
  refreshTaskId = $state<string | null>(null);
  refreshDetectedUrl = $state<string | null>(null);
  refreshError = $state<string | null>(null);
  refreshCountdown = $state<number>(3);

  isDuplicateModalOpen = $state<boolean>(false);
  duplicateModalData = $state<DuplicateCheckResult | null>(null);
  duplicateModalUrl = $state<string>('');
  duplicateModalHeaders = $state<Record<string, string> | null>(null);

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

  refreshTask = $derived<DownloadTask | null>(
    this.tasks.find((t) => t.id === this.refreshTaskId) || null
  );


  filteredTasks = $derived.by<DownloadTask[]>(() => {
    const list = this.tasks.filter((task) => {
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

    const factor = this.sortOrder === 'asc' ? 1 : -1;

    list.sort((a, b) => {
      if (this.sortBy === 'name') {
        return factor * a.filename.localeCompare(b.filename, undefined, { numeric: true, sensitivity: 'base' });
      }
      if (this.sortBy === 'size') {
        const sizeA = a.total_bytes ?? a.downloaded_bytes;
        const sizeB = b.total_bytes ?? b.downloaded_bytes;
        return factor * (sizeA - sizeB);
      }
      if (this.sortBy === 'progress') {
        const pctA = a.total_bytes && a.total_bytes > 0 ? a.downloaded_bytes / a.total_bytes : 0;
        const pctB = b.total_bytes && b.total_bytes > 0 ? b.downloaded_bytes / b.total_bytes : 0;
        return factor * (pctA - pctB);
      }
      if (this.sortBy === 'speed') {
        const speedA = a.speed_bps ?? 0;
        const speedB = b.speed_bps ?? 0;
        return factor * (speedA - speedB);
      }
      if (this.sortBy === 'status') {
        const statusA = typeof a.status === 'string' ? a.status : 'failed';
        const statusB = typeof b.status === 'string' ? b.status : 'failed';
        return factor * statusA.localeCompare(statusB);
      }
      // Default: 'date'
      const dateA = new Date(a.created_at).getTime() || 0;
      const dateB = new Date(b.created_at).getTime() || 0;
      return factor * (dateA - dateB);
    });

    return list;
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
    try {
      const v = await fetchRuntimeAppVersion();
      if (v) this.appVersion = v;
    } catch {
      // fallback to compile time version
    }

    await this.loadAppSettings();

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
            error_message: null,
            segments: [],
            speed_bps: 0,
            eta_seconds: 0,
            referer: "https://x.com/lusthunter/status/18800000",
            speed_limit_bps: null,
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
            completed_at: null,
            error_message: null,
            segments: [],
            speed_bps: 12500000,
            eta_seconds: 103,
            referer: "https://releases.ubuntu.com/22.04/",
            speed_limit_bps: 1048576, // 1 MB/s limit
          },
          {
            id: 'mock-3',
            url: "https://expired-cdn.example.com/interrupted-archive.zip",
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
            completed_at: null,
            error_message: "HTTP 504 Gateway Timeout / Connection Refused by Host Server",
            segments: [],
            speed_bps: 0,
            eta_seconds: 0,
            referer: "https://example.com/download-archive-page",
            speed_limit_bps: null,
          },
          {
            id: 'mock-4',
            url: "https://cdn.example.com/dataset-large.tar.gz?token=exp123",
            filename: "dataset-large.tar.gz",
            save_dir: "D:\\IDM_Downloads\\Compressed",
            file_path: "D:\\IDM_Downloads\\Compressed\\dataset-large.tar.gz",
            total_bytes: 1073741824,
            downloaded_bytes: 536870912,
            category: "compressed",
            status: "paused",
            connections: 8,
            supports_range: true,
            is_hls: false,
            created_at: "2026-09-20 13:00:00",
            completed_at: null,
            error_message: null,
            segments: [],
            speed_bps: 0,
            eta_seconds: 0,
            referer: "https://example.com/datasets",
            speed_limit_bps: null,
          },
        ];
        this.selectedTaskId = 'mock-1';
      }
      return;
    }
    await this.refreshTasks();
    await this.setupEventListeners();
    try {
      const cfg = await invoke<GlobalSpeedLimitConfig>('get_global_speed_limit');
      if (cfg) {
        this.speedLimiterEnabled = cfg.enabled;
        if (cfg.limit_bps) {
          const parsed = bpsToUnit(cfg.limit_bps);
          this.globalSpeedLimitValue = parsed.value;
          this.globalSpeedLimitUnit = parsed.unit;
        }
      }
    } catch (e) {
      console.error('Failed to load global speed limit config:', e);
    }
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

      if (this.settings.notifyOnComplete && typeof Notification !== 'undefined') {
        try {
          if (Notification.permission === 'granted') {
            const finishedTask = this.tasks[idx];
            new Notification(this.t('notify.completedTitle'), {
              body: finishedTask ? finishedTask.filename : this.t('notify.completedBody'),
              icon: '/favicon.png',
            });
          }
        } catch {
          // Notification failed
        }
      }
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
      let errMsg = this.t('notify.failedDefault');
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
    await listen<any>('browser-download-requested', async (event) => {
      console.log('browser-download-requested received:', event.payload);
      const p = event.payload;
      if (p && p.url) {
        if (this.isRefreshModalOpen && this.refreshTaskId) {
          this.refreshDetectedUrl = p.url;
        } else {
          const dup = await this.checkDuplicateDownload(p.url, p.filename || '');
          if (dup.is_duplicate) {
            this.openDuplicateModal(dup, p.url, p.headers || null);
          } else {
            this.openAddModal(p.url, p.filename, p.headers || null);
          }
        }
      }
    });

    // 6. Clipboard watcher on window focus
    if (typeof window !== 'undefined' && typeof window.addEventListener === 'function') {
      window.addEventListener('focus', () => {
        this.checkClipboardForUrl();
      });
    }
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

  async openTransferWindow(taskId?: string): Promise<void> {
    const id = taskId || this.selectedTaskId || this.progressModalTaskId;
    if (!id) return;
    this.progressModalTaskId = id;
    this.selectedTaskId = id;
    if (isTauri()) {
      try {
        await invoke('open_transfer_window', { taskId: id });
        return;
      } catch (e) {
        console.warn('Failed to open native transfer window, falling back to modal:', e);
      }
    }
    this.isProgressModalOpen = true;
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

  setSort(criterion: SortCriterion, order?: SortOrder) {
    if (this.sortBy === criterion && !order) {
      this.sortOrder = this.sortOrder === 'asc' ? 'desc' : 'asc';
    } else {
      this.sortBy = criterion;
      if (order) {
        this.sortOrder = order;
      } else {
        this.sortOrder = criterion === 'name' ? 'asc' : 'desc';
      }
    }
  }

  toggleSortOrder() {
    this.sortOrder = this.sortOrder === 'asc' ? 'desc' : 'asc';
  }

  cycleSortCriteria() {
    const criteria: SortCriterion[] = ['date', 'size', 'name', 'progress', 'speed', 'status'];
    const idx = criteria.indexOf(this.sortBy);
    const nextIdx = (idx + 1) % criteria.length;
    this.sortBy = criteria[nextIdx];
  }

  t(key: TranslationKey, params?: Record<string, string | number>): string {
    return getTranslation(this.settings.language, key, params);
  }

  async setLanguage(lang: SupportedLanguage) {
    await this.saveAppSettings({ language: lang });
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

  startRefreshLink(taskId: string) {
    const task = this.tasks.find((t) => t.id === taskId);
    if (!task) return;

    this.refreshTaskId = taskId;
    this.refreshDetectedUrl = null;
    this.refreshError = null;
    this.refreshCountdown = 3;
    this.isRefreshModalOpen = true;

    // Close other modals if open
    this.isOutcomeModalOpen = false;
    this.isPropertiesModalOpen = false;
    this.isProgressModalOpen = false;

    // Open source web page in browser if referer or url exists
    const targetUrl = task.referer || task.url;
    if (targetUrl && (targetUrl.startsWith('http://') || targetUrl.startsWith('https://'))) {
      this.openExternalUrl(targetUrl);
    }
  }

  cancelRefreshLink() {
    this.isRefreshModalOpen = false;
    this.refreshTaskId = null;
    this.refreshDetectedUrl = null;
    this.refreshError = null;
  }

  async applyRefreshedUrl(newUrl: string) {
    if (!this.refreshTaskId) return;
    const taskId = this.refreshTaskId;
    this.refreshError = null;

    try {
      if (isTauri()) {
        await invoke('refresh_download_url', { taskId, newUrl });
      }

      const idx = this.tasks.findIndex((t) => t.id === taskId);
      if (idx !== -1) {
        this.tasks[idx].url = newUrl;
        this.tasks[idx].error_message = null;
        if (typeof this.tasks[idx].status === 'object' && 'failed' in this.tasks[idx].status) {
          this.tasks[idx].status = 'paused';
        }
      }

      this.cancelRefreshLink();
      await this.resumeTask(taskId);
    } catch (e: any) {
      console.error('Failed to refresh download URL:', e);
      this.refreshError = typeof e === 'string' ? e : (e?.message || this.t('notify.refreshFailedDefault'));
      throw e;
    }
  }

  async reportDiagnostic(taskId: string, errorMessage?: string): Promise<string> {
    if (!isTauri()) {
      return `local-diag-${taskId.slice(0, 8)}`;
    }
    try {
      const eventId = await invoke<string>('report_diagnostic_error', {
        taskId,
        errorMessage: errorMessage || null,
      });
      return eventId;
    } catch (e: any) {
      console.error('Failed to report diagnostic error:', e);
      throw e;
    }
  }

  async checkDuplicateDownload(url: string, filename: string = '', saveDir: string = ''): Promise<DuplicateCheckResult> {

    if (!url.trim()) {
      return {
        is_duplicate: false,
        status: null,
        task_id: null,
        filename: null,
        file_path: null,
        file_exists_on_disk: false,
        downloaded_bytes: 0,
        total_bytes: null,
        percent: 0,
        completed_at: null,
        suggested_new_filename: null,
      };
    }
    if (isTauri()) {
      try {
        const res = await invoke<DuplicateCheckResult>('check_duplicate_download', {
          url: url.trim(),
          filename: filename.trim(),
          saveDir: saveDir.trim(),
        });
        if (res) return res;
      } catch (e) {
        console.warn('Failed to check duplicate download via Tauri invoke:', e);
      }
    }

    // In-memory fallback
    const trimmed = url.trim();
    const matched = this.tasks.find((t) => t.url === trimmed || (filename && t.filename === filename));
    if (matched) {
      const isCompleted = matched.status === 'completed';
      const pct = matched.total_bytes ? Math.min(100, (matched.downloaded_bytes / matched.total_bytes) * 100) : (isCompleted ? 100 : 0);
      const extMatch = (matched.filename || 'file').match(/^(.*?)(?:\.([^.]+))?$/);
      const stem = extMatch ? extMatch[1] : (matched.filename || 'file');
      const ext = extMatch && extMatch[2] ? `.${extMatch[2]}` : '';
      return {
        is_duplicate: true,
        status: matched.status,
        task_id: matched.id,
        filename: matched.filename,
        file_path: matched.file_path,
        file_exists_on_disk: true,
        downloaded_bytes: matched.downloaded_bytes,
        total_bytes: matched.total_bytes,
        percent: pct,
        completed_at: matched.completed_at,
        suggested_new_filename: `${stem} (1)${ext}`,
      };
    }

    return {
      is_duplicate: false,
      status: null,
      task_id: null,
      filename: null,
      file_path: null,
      file_exists_on_disk: false,
      downloaded_bytes: 0,
      total_bytes: null,
      percent: 0,
      completed_at: null,
      suggested_new_filename: null,
    };
  }

  async openDuplicateModal(data: DuplicateCheckResult, url: string, headers: Record<string, string> | null = null) {
    this.duplicateModalData = data;
    this.duplicateModalUrl = url;
    this.duplicateModalHeaders = headers;

    if (this.settings.duplicateActionRemember && this.settings.duplicateAction && this.settings.duplicateAction !== 'ask') {
      await this.proceedWithDuplicateAction(this.settings.duplicateAction, false);
      return;
    }

    this.isDuplicateModalOpen = true;
    this.isAddModalOpen = false;
  }

  closeDuplicateModal() {
    this.isDuplicateModalOpen = false;
    this.duplicateModalData = null;
    this.duplicateModalUrl = '';
    this.duplicateModalHeaders = null;
  }

  async proceedWithDuplicateAction(choice: 'numbered' | 'overwrite' | 'resume', remember: boolean = false) {
    const data = this.duplicateModalData;
    const url = this.duplicateModalUrl;
    const headers = this.duplicateModalHeaders;

    if (remember) {
      this.settings.duplicateAction = choice;
      this.settings.duplicateActionRemember = true;
      await this.saveAppSettings({ duplicateAction: choice, duplicateActionRemember: true });
    }

    this.closeDuplicateModal();

    if (choice === 'numbered') {
      const filename = data?.suggested_new_filename || '';
      this.openAddModal(url, filename, headers);
    } else if (choice === 'overwrite') {
      const filename = data?.filename || '';
      this.openAddModal(url, filename, headers);
    } else if (choice === 'resume') {
      if (data?.task_id) {
        if (data.status === 'paused') {
          await this.resumeTask(data.task_id);
        }
        await this.openTransferWindow(data.task_id);
      } else {
        this.openAddModal(url, data?.filename || '', headers);
      }
    }
  }

  proceedWithNewDownload() {
    this.proceedWithDuplicateAction('numbered', false);
  }

  async openExternalUrl(url: string) {
    try {
      if (isTauri()) {
        await invoke('open_external_url', { url });
      } else {
        window.open(url, '_blank');
      }
    } catch (e) {
      console.warn('Failed to open external URL:', e);
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

  async setGlobalSpeedLimit(enabled: boolean, value?: number, unit?: SpeedLimitUnit) {
    this.speedLimiterEnabled = enabled;
    if (value !== undefined && !isNaN(value) && value > 0) {
      this.globalSpeedLimitValue = value;
    }
    if (unit !== undefined) {
      this.globalSpeedLimitUnit = unit;
    }

    const bps = this.speedLimiterEnabled ? unitToBps(this.globalSpeedLimitValue, this.globalSpeedLimitUnit) : null;
    if (isTauri()) {
      try {
        await invoke('set_global_speed_limit', {
          enabled: this.speedLimiterEnabled,
          limitBps: bps,
        });
      } catch (e) {
        console.error('Failed to set global speed limit:', e);
      }
    }
  }

  async setTaskSpeedLimit(taskId: string, limitBps: number | null) {
    const idx = this.tasks.findIndex((t) => t.id === taskId);
    if (idx !== -1) {
      this.tasks[idx].speed_limit_bps = limitBps;
    }
    if (isTauri()) {
      try {
        await invoke('set_task_speed_limit', {
          taskId,
          limitBps,
        });
      } catch (e) {
        console.error('Failed to set task speed limit:', e);
      }
    }
  }

  applyRawSettings(raw: Record<string, string>) {
    const s = { ...this.settings };
    if ('language' in raw && (raw.language === 'id' || raw.language === 'en')) {
      s.language = raw.language;
    }
    if ('autoStartWindows' in raw) s.autoStartWindows = raw.autoStartWindows === 'true';
    if ('mediaPanelOverlay' in raw) s.mediaPanelOverlay = raw.mediaPanelOverlay === 'true';
    if ('clipboardAutoCapture' in raw) s.clipboardAutoCapture = raw.clipboardAutoCapture === 'true';
    if ('notifyOnComplete' in raw) s.notifyOnComplete = raw.notifyOnComplete === 'true';
    if ('connectionType' in raw && raw.connectionType) s.connectionType = raw.connectionType;
    if ('defaultConnections' in raw) {
      const num = parseInt(raw.defaultConnections, 10);
      if (!isNaN(num) && num > 0) s.defaultConnections = num;
    }
    if ('tcpWindowAutoTuning' in raw) s.tcpWindowAutoTuning = raw.tcpWindowAutoTuning === 'true';
    if ('browserChrome' in raw) s.browserChrome = raw.browserChrome === 'true';
    if ('browserEdge' in raw) s.browserEdge = raw.browserEdge === 'true';
    if ('browserFirefox' in raw) s.browserFirefox = raw.browserFirefox === 'true';
    if ('browserBrave' in raw) s.browserBrave = raw.browserBrave === 'true';
    if ('defaultDownloadDir' in raw) s.defaultDownloadDir = raw.defaultDownloadDir;
    if ('categorySubfolders' in raw) s.categorySubfolders = raw.categorySubfolders === 'true';
    if ('tempDir' in raw) s.tempDir = raw.tempDir;
    if ('autoCaptureExtensions' in raw) s.autoCaptureExtensions = raw.autoCaptureExtensions;
    if ('excludedSites' in raw) s.excludedSites = raw.excludedSites;
    if ('connectionTimeoutSec' in raw) {
      const num = parseInt(raw.connectionTimeoutSec, 10);
      if (!isNaN(num) && num > 0) s.connectionTimeoutSec = num;
    }
    if ('maxRetries' in raw) {
      const num = parseInt(raw.maxRetries, 10);
      if (!isNaN(num) && num >= 0) s.maxRetries = num;
    }
    if ('duplicateAction' in raw && (raw.duplicateAction === 'ask' || raw.duplicateAction === 'numbered' || raw.duplicateAction === 'overwrite' || raw.duplicateAction === 'resume')) {
      s.duplicateAction = raw.duplicateAction;
    }
    if ('duplicateActionRemember' in raw) {
      s.duplicateActionRemember = raw.duplicateActionRemember === 'true';
    }
    this.settings = s;
  }

  settingsToRaw(s: AppSettings): Record<string, string> {
    return {
      language: s.language || 'id',
      autoStartWindows: String(s.autoStartWindows),
      mediaPanelOverlay: String(s.mediaPanelOverlay),
      clipboardAutoCapture: String(s.clipboardAutoCapture),
      notifyOnComplete: String(s.notifyOnComplete),
      connectionType: s.connectionType,
      defaultConnections: String(s.defaultConnections),
      tcpWindowAutoTuning: String(s.tcpWindowAutoTuning),
      browserChrome: String(s.browserChrome),
      browserEdge: String(s.browserEdge),
      browserFirefox: String(s.browserFirefox),
      browserBrave: String(s.browserBrave),
      defaultDownloadDir: s.defaultDownloadDir || '',
      categorySubfolders: String(s.categorySubfolders),
      tempDir: s.tempDir || '',
      autoCaptureExtensions: s.autoCaptureExtensions,
      excludedSites: s.excludedSites || '',
      connectionTimeoutSec: String(s.connectionTimeoutSec),
      maxRetries: String(s.maxRetries),
      duplicateAction: s.duplicateAction || 'ask',
      duplicateActionRemember: String(s.duplicateActionRemember ?? false),
    };
  }

  async loadAppSettings() {
    try {
      if (isTauri()) {
        const raw = await invoke<Record<string, string>>('get_app_settings');
        if (raw && Object.keys(raw).length > 0) {
          this.applyRawSettings(raw);
        }
        if (!this.settings.defaultDownloadDir) {
          try {
            const defDir = await invoke<string>('get_default_download_dir');
            if (defDir) this.settings.defaultDownloadDir = defDir;
          } catch {}
        }
      } else if (typeof window !== 'undefined' && window.localStorage) {
        const saved = window.localStorage.getItem('myownidm_settings');
        if (saved) {
          this.settings = { ...DEFAULT_APP_SETTINGS, ...JSON.parse(saved) };
        }
      }
    } catch (e) {
      console.warn('Failed to load settings:', e);
    }
  }

  async saveAppSettings(newSettings: Partial<AppSettings>) {
    this.settings = { ...this.settings, ...newSettings };
    if (isTauri()) {
      try {
        const raw = this.settingsToRaw(this.settings);
        await invoke('save_app_settings', { settings: raw });
      } catch (e) {
        console.error('Failed to save settings to DB:', e);
      }
    } else if (typeof window !== 'undefined' && window.localStorage) {
      try {
        window.localStorage.setItem('myownidm_settings', JSON.stringify(this.settings));
      } catch {}
    }
  }

  async resetAppSettings() {
    let defDir = this.settings.defaultDownloadDir;
    if (!defDir && isTauri()) {
      try {
        defDir = await invoke<string>('get_default_download_dir');
      } catch {}
    }
    const defaults: AppSettings = {
      ...DEFAULT_APP_SETTINGS,
      defaultDownloadDir: defDir || DEFAULT_APP_SETTINGS.defaultDownloadDir,
    };
    await this.saveAppSettings(defaults);
  }

  async checkClipboardForUrl() {
    if (!this.settings.clipboardAutoCapture) return;
    if (this.isAddModalOpen) return;
    if (typeof navigator === 'undefined' || !navigator.clipboard || !navigator.clipboard.readText) return;
    try {
      const text = await navigator.clipboard.readText();
      if (!text || typeof text !== 'string') return;
      const trimmed = text.trim();
      if (trimmed === this.lastCheckedClipboard) return;
      this.lastCheckedClipboard = trimmed;
      if (matchesDownloadExtension(trimmed, this.settings.autoCaptureExtensions)) {
        this.openAddModal(trimmed);
      }
    } catch {
      // Clipboard read denied or unavailable
    }
  }
}

export const store = new IdmStore();

