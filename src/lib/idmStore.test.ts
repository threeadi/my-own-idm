import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { DownloadTask, SpeedMetrics, DuplicateCheckResult } from './types';

// Mock Tauri APIs
const mockInvoke = vi.fn();
const eventListeners = new Map<string, (event: { payload: any }) => void>();

let mockIsTauriReturn = true;

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: any[]) => mockInvoke(...args),
  isTauri: () => mockIsTauriReturn,
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: (eventName: string, callback: (event: { payload: any }) => void) => {
    eventListeners.set(eventName, callback);
    return Promise.resolve(() => eventListeners.delete(eventName));
  },
}));

// Import after mocks
import { IdmStore, isTauri } from './idmStore.svelte';

function makeTask(overrides: Partial<DownloadTask> = {}): DownloadTask {
  return {
    id: 'test-1',
    url: 'https://example.com/video.mp4',
    filename: 'video.mp4',
    save_dir: 'C:\\Downloads',
    file_path: 'C:\\Downloads\\video.mp4',
    total_bytes: 1048576,
    downloaded_bytes: 524288,
    category: 'video',
    status: 'downloading',
    connections: 4,
    supports_range: true,
    is_hls: false,
    created_at: '2026-09-20 12:00:00',
    completed_at: null,
    error_message: null,
    segments: [],
    speed_bps: 100000,
    eta_seconds: 5,
    ...overrides,
  };
}

describe('IdmStore State & Filtering', () => {
  beforeEach(() => {
    (globalThis as any).window = {
      __TAURI_INTERNALS__: {},
      isTauri: true,
      addEventListener: vi.fn(),
    };
    mockInvoke.mockReset();
    eventListeners.clear();
  });


  it('initializes with default empty state', () => {
    const store = new IdmStore();
    expect(store.tasks).toEqual([]);
    expect(store.selectedTaskId).toBeNull();
    expect(store.activeCategory).toBe('all');
    expect(store.searchQuery).toBe('');
    expect(store.isAddModalOpen).toBe(false);
    expect(store.appVersion).toBe('0.1.0-dev');
  });

  it('filters tasks by category', () => {
    const store = new IdmStore();
    store.tasks = [
      makeTask({ id: '1', filename: 'clip.mp4', category: 'video' }),
      makeTask({ id: '2', filename: 'archive.zip', category: 'compressed' }),
      makeTask({ id: '3', filename: 'paper.pdf', category: 'documents' }),
    ];

    store.activeCategory = 'video';
    expect(store.filteredTasks.length).toBe(1);
    expect(store.filteredTasks[0].id).toBe('1');

    store.activeCategory = 'all';
    expect(store.filteredTasks.length).toBe(3);
  });

  it('filters tasks by status', () => {
    const store = new IdmStore();
    store.tasks = [
      makeTask({ id: '1', status: 'downloading' }),
      makeTask({ id: '2', status: 'completed' }),
      makeTask({ id: '3', status: 'paused' }),
    ];

    store.activeCategory = 'downloading';
    expect(store.filteredTasks.length).toBe(1);
    expect(store.filteredTasks[0].id).toBe('1');

    store.activeCategory = 'completed';
    expect(store.filteredTasks.length).toBe(1);
    expect(store.filteredTasks[0].id).toBe('2');

    store.activeCategory = 'paused';
    expect(store.filteredTasks.length).toBe(1);
    expect(store.filteredTasks[0].id).toBe('3');
  });

  it('filters tasks by search query', () => {
    const store = new IdmStore();
    store.tasks = [
      makeTask({ id: '1', filename: 'Avatar_Trailer.mp4', url: 'https://youtube.com/watch' }),
      makeTask({ id: '2', filename: 'Rust_Book.pdf', url: 'https://rust-lang.org/book' }),
    ];

    store.searchQuery = 'avatar';
    expect(store.filteredTasks.length).toBe(1);
    expect(store.filteredTasks[0].id).toBe('1');

    store.searchQuery = 'rust-lang';
    expect(store.filteredTasks.length).toBe(1);
    expect(store.filteredTasks[0].id).toBe('2');

    store.searchQuery = 'nonexistent';
    expect(store.filteredTasks.length).toBe(0);
  });

  it('computes categoryCounts accurately', () => {
    const store = new IdmStore();
    store.tasks = [
      makeTask({ id: '1', category: 'video', status: 'downloading' }),
      makeTask({ id: '2', category: 'video', status: 'completed' }),
      makeTask({ id: '3', category: 'documents', status: 'paused' }),
    ];

    const counts = store.categoryCounts;
    expect(counts.all).toBe(3);
    expect(counts.video).toBe(2);
    expect(counts.documents).toBe(1);
    expect(counts.downloading).toBe(1);
    expect(counts.completed).toBe(1);
    expect(counts.paused).toBe(1);
  });

  it('computes totalSpeedBps accurately', () => {
    const store = new IdmStore();
    store.tasks = [
      makeTask({ id: '1', status: 'downloading', speed_bps: 500000 }),
      makeTask({ id: '2', status: 'downloading', speed_bps: 250000 }),
      makeTask({ id: '3', status: 'paused', speed_bps: 999999 }), // Should not count paused
    ];

    expect(store.totalSpeedBps).toBe(750000);
  });

  it('opens add modal with prefilled data', () => {
    const store = new IdmStore();
    store.openAddModal('https://example.com/file.iso', 'file.iso', { Authorization: 'Bearer token' });

    expect(store.isAddModalOpen).toBe(true);
    expect(store.initialAddUrl).toBe('https://example.com/file.iso');
    expect(store.initialFilename).toBe('file.iso');
    expect(store.initialHeaders).toEqual({ Authorization: 'Bearer token' });
  });

  it('verifies isTauri helper', () => {
    // In node environment, window may be undefined or mock
    expect(typeof isTauri()).toBe('boolean');
  });

  it('refreshes tasks from Tauri backend', async () => {
    const store = new IdmStore();
    const taskList = [
      makeTask({ id: 't1', filename: 't1.mp4' }),
      makeTask({ id: 't2', filename: 't2.zip' }),
    ];
    mockInvoke.mockResolvedValueOnce(taskList);

    await store.refreshTasks();
    expect(mockInvoke).toHaveBeenCalledWith('get_all_tasks');
    expect(store.tasks.length).toBe(2);
    expect(store.selectedTaskId).toBe('t1');
    expect(store.selectedTask?.filename).toBe('t1.mp4');
  });

  it('handles refreshTasks error gracefully', async () => {
    const store = new IdmStore();
    mockInvoke.mockRejectedValueOnce(new Error('IPC failed'));
    await store.refreshTasks();
    expect(store.tasks.length).toBe(0);
  });

  it('initializes store and hooks event listeners', async () => {
    const store = new IdmStore();
    mockInvoke.mockResolvedValueOnce([]);

    await store.init();
    expect(eventListeners.has('download-progress')).toBe(true);
    expect(eventListeners.has('download-completed')).toBe(true);
    expect(eventListeners.has('download-paused')).toBe(true);
    expect(eventListeners.has('download-failed')).toBe(true);
    expect(eventListeners.has('browser-download-requested')).toBe(true);
  });

  it('handles init in non-Tauri preview environment', async () => {
    delete (globalThis as any).window;
    const store = new IdmStore();
    await store.init(); // Should log info and load preview mock tasks
    expect(store.tasks.length).toBe(4);
    expect(store.tasks[0].id).toBe('mock-1');
  });

  it('updates task on download-progress event', async () => {
    const store = new IdmStore();
    store.tasks = [makeTask({ id: 'task-prog', downloaded_bytes: 0, speed_bps: 0 })];

    await store.setupEventListeners();

    const progressHandler = eventListeners.get('download-progress');
    expect(progressHandler).toBeDefined();

    const progressPayload: SpeedMetrics = {
      task_id: 'task-prog',
      downloaded_bytes: 500000,
      total_bytes: 1000000,
      speed_bps: 250000,
      eta_seconds: 2,
      percent: 50.0,
      status: 'downloading',
      segments: [],
    };
    progressHandler!({ payload: progressPayload });

    expect(store.tasks[0].downloaded_bytes).toBe(500000);
    expect(store.tasks[0].speed_bps).toBe(250000);
    expect(store.tasks[0].eta_seconds).toBe(2);

    // Test progress when status becomes completed
    const completedProgressPayload: SpeedMetrics = {
      task_id: 'task-prog',
      downloaded_bytes: 1000000,
      total_bytes: 1000000,
      speed_bps: 0,
      eta_seconds: 0,
      percent: 100.0,
      status: 'completed',
      segments: [],
    };
    progressHandler!({ payload: completedProgressPayload });
    expect(store.tasks[0].status).toBe('completed');
    expect(store.tasks[0].downloaded_bytes).toBe(1000000);
  });

  it('updates task on download-completed event and triggers outcome modal', async () => {
    const store = new IdmStore();
    store.tasks = [makeTask({ id: 'task-comp', status: 'downloading', speed_bps: 50000, total_bytes: 5000, downloaded_bytes: 2500 })];
    store.isProgressModalOpen = true;
    store.progressModalTaskId = 'task-comp';

    await store.setupEventListeners();
    const handler = eventListeners.get('download-completed');
    handler!({ payload: 'task-comp' });

    expect(store.tasks[0].status).toBe('completed');
    expect(store.tasks[0].speed_bps).toBe(0);
    expect(store.tasks[0].eta_seconds).toBe(0);
    expect(store.tasks[0].downloaded_bytes).toBe(5000);
    expect(store.isProgressModalOpen).toBe(false);
    expect(store.isOutcomeModalOpen).toBe(true);
    expect(store.outcomeType).toBe('completed');
    expect(store.outcomeTaskId).toBe('task-comp');
    expect(store.outcomeTask?.id).toBe('task-comp');
  });

  it('updates task on download-paused event', async () => {
    const store = new IdmStore();
    store.tasks = [makeTask({ id: 'task-pause', status: 'downloading', speed_bps: 50000 })];

    await store.setupEventListeners();
    const handler = eventListeners.get('download-paused');
    handler!({ payload: 'task-pause' });

    expect(store.tasks[0].status).toBe('paused');
    expect(store.tasks[0].speed_bps).toBe(0);
  });

  it('updates task on download-failed event and triggers outcome modal', async () => {
    const store = new IdmStore();
    store.tasks = [makeTask({ id: 'task-fail', status: 'downloading', speed_bps: 50000 })];
    store.isProgressModalOpen = true;
    store.progressModalTaskId = 'task-fail';

    await store.setupEventListeners();
    const handler = eventListeners.get('download-failed');
    handler!({ payload: 'task-fail' });

    expect(store.tasks[0].status).toEqual({ failed: 'Gagal mengunduh berkas atau koneksi terputus.' });
    expect(store.tasks[0].speed_bps).toBe(0);
    expect(store.isProgressModalOpen).toBe(false);
    expect(store.isOutcomeModalOpen).toBe(true);
    expect(store.outcomeType).toBe('failed');
    expect(store.outcomeTaskId).toBe('task-fail');
  });

  it('handles browser-download-requested event', async () => {
    const store = new IdmStore();
    await store.setupEventListeners();
    const handler = eventListeners.get('browser-download-requested');

    mockInvoke.mockResolvedValueOnce({ is_duplicate: false });

    await handler!({
      payload: {
        url: 'https://youtube.com/watch?v=123',
        filename: 'video.mp4',
        headers: { 'User-Agent': 'Custom' },
      },
    });

    expect(store.isAddModalOpen).toBe(true);
    expect(store.initialAddUrl).toBe('https://youtube.com/watch?v=123');
    expect(store.initialFilename).toBe('video.mp4');
    expect(store.initialHeaders).toEqual({ 'User-Agent': 'Custom' });
  });

  it('pauses and resumes download task', async () => {
    const store = new IdmStore();
    store.tasks = [makeTask({ id: 't-ctrl', status: 'downloading', speed_bps: 50000 })];

    mockInvoke.mockResolvedValue(undefined);

    await store.pauseTask('t-ctrl');
    expect(mockInvoke).toHaveBeenCalledWith('pause_download', { taskId: 't-ctrl' });
    expect(store.tasks[0].status).toBe('paused');
    expect(store.tasks[0].speed_bps).toBe(0);

    await store.resumeTask('t-ctrl');
    expect(mockInvoke).toHaveBeenCalledWith('resume_download', { taskId: 't-ctrl' });
    expect(store.tasks[0].status).toBe('downloading');
  });

  it('cancels download task and cleans selection', async () => {
    const store = new IdmStore();
    store.tasks = [
      makeTask({ id: 't1', filename: 't1.mp4' }),
      makeTask({ id: 't2', filename: 't2.mp4' }),
    ];
    store.selectedTaskId = 't1';

    mockInvoke.mockResolvedValue(undefined);

    await store.cancelTask('t1', true);
    expect(mockInvoke).toHaveBeenCalledWith('cancel_download', { taskId: 't1', deleteFile: true });
    expect(store.tasks.length).toBe(1);
    expect(store.tasks[0].id).toBe('t2');
    expect(store.selectedTaskId).toBe('t2');
  });

  it('opens file and folder via Tauri invoke', async () => {
    const store = new IdmStore();
    mockInvoke.mockResolvedValue(undefined);

    await store.openFile('C:\\Downloads\\test.mp4');
    expect(mockInvoke).toHaveBeenCalledWith('open_file', { path: 'C:\\Downloads\\test.mp4' });

    await store.openFolder('C:\\Downloads\\test.mp4');
    expect(mockInvoke).toHaveBeenCalledWith('open_file_in_folder', { path: 'C:\\Downloads\\test.mp4' });
  });

  it('handles IPC errors gracefully in pause, resume, cancel, openFile, openFolder', async () => {
    const store = new IdmStore();
    store.tasks = [makeTask({ id: 't-err' })];
    mockInvoke.mockRejectedValue(new Error('Simulated backend error'));

    // None of these should throw uncaught exceptions
    await store.pauseTask('t-err');
    await store.resumeTask('t-err');
    await store.cancelTask('t-err');
    await store.openFile('invalid');
    await store.openFolder('invalid');
  });

  it('manages progress modal state and target task ID', () => {
    const store = new IdmStore();
    expect(store.isProgressModalOpen).toBe(false);
    expect(store.progressModalTaskId).toBeNull();

    store.openProgressModal('task-prog-1');
    expect(store.isProgressModalOpen).toBe(true);
    expect(store.progressModalTaskId).toBe('task-prog-1');
    expect(store.selectedTaskId).toBe('task-prog-1');

    store.closeProgressModal();
    expect(store.isProgressModalOpen).toBe(false);
    expect(store.progressModalTaskId).toBeNull();
  });

  it('manages openTransferWindow with Tauri invoke and fallback', async () => {
    const store = new IdmStore();
    mockInvoke.mockResolvedValue(undefined);

    // 1. Success with explicit taskId
    await store.openTransferWindow('task-win-1');
    expect(mockInvoke).toHaveBeenCalledWith('open_transfer_window', { taskId: 'task-win-1' });
    expect(store.progressModalTaskId).toBe('task-win-1');
    expect(store.selectedTaskId).toBe('task-win-1');

    // 2. Invoking with selectedTaskId fallback
    store.selectedTaskId = 'task-win-2';
    await store.openTransferWindow();
    expect(mockInvoke).toHaveBeenCalledWith('open_transfer_window', { taskId: 'task-win-2' });

    // 3. Fallback to modal when invoke fails
    mockInvoke.mockRejectedValueOnce(new Error('Tauri window error'));
    await store.openTransferWindow('task-win-3');
    expect(store.isProgressModalOpen).toBe(true);

    // 4. Return early when no id is available
    store.selectedTaskId = null;
    store.progressModalTaskId = null;
    await store.openTransferWindow();
  });

  it('handles batch operations: resumeAll, pauseAll, clearCompleted', async () => {
    const store = new IdmStore();
    store.tasks = [
      makeTask({ id: 't-paused-1', status: 'paused' }),
      makeTask({ id: 't-failed-1', status: { failed: 'network error' } as any }),
      makeTask({ id: 't-active-1', status: 'downloading' }),
      makeTask({ id: 't-active-2', status: 'downloading' }),
      makeTask({ id: 't-done-1', status: 'completed' }),
    ];

    mockInvoke.mockResolvedValue(undefined);

    // Test resumeAll: should resume paused and failed tasks
    await store.resumeAll();
    expect(mockInvoke).toHaveBeenCalledWith('resume_download', { taskId: 't-paused-1' });
    expect(mockInvoke).toHaveBeenCalledWith('resume_download', { taskId: 't-failed-1' });

    // Test pauseAll: should pause downloading tasks
    mockInvoke.mockClear();
    await store.pauseAll();
    expect(mockInvoke).toHaveBeenCalledWith('pause_download', { taskId: 't-active-1' });
    expect(mockInvoke).toHaveBeenCalledWith('pause_download', { taskId: 't-active-2' });

    // Test clearCompleted: should cancel completed tasks without deleting files
    mockInvoke.mockClear();
    await store.clearCompleted();
    expect(mockInvoke).toHaveBeenCalledWith('cancel_download', { taskId: 't-done-1', deleteFile: false });
  });

  it('supports Stitch UI customization properties', () => {
    const store = new IdmStore();
    expect(store.viewMode).toBe('cards');
    store.viewMode = 'table';
    expect(store.viewMode).toBe('table');

    expect(store.sortBy).toBe('date');
    store.sortBy = 'size';
    expect(store.sortBy).toBe('size');
    store.sortBy = 'name';
    expect(store.sortBy).toBe('name');

    expect(store.speedLimiterEnabled).toBe(false);
    store.speedLimiterEnabled = true;
    expect(store.speedLimiterEnabled).toBe(true);
  });

  it('manages outcome modal open and close methods', () => {
    const store = new IdmStore();
    expect(store.isOutcomeModalOpen).toBe(false);

    store.openOutcomeModal('task-123', 'completed');
    expect(store.isOutcomeModalOpen).toBe(true);
    expect(store.outcomeTaskId).toBe('task-123');
    expect(store.outcomeType).toBe('completed');
    expect(store.outcomeErrorMessage).toBeNull();

    store.openOutcomeModal('task-456', 'failed', 'Connection timeout');
    expect(store.isOutcomeModalOpen).toBe(true);
    expect(store.outcomeTaskId).toBe('task-456');
    expect(store.outcomeType).toBe('failed');
    expect(store.outcomeErrorMessage).toBe('Connection timeout');

    store.closeOutcomeModal();
    expect(store.isOutcomeModalOpen).toBe(false);
    expect(store.outcomeTaskId).toBeNull();
    expect(store.outcomeErrorMessage).toBeNull();
  });

  it('manages properties modal open and close methods', () => {
    const store = new IdmStore();
    expect(store.isPropertiesModalOpen).toBe(false);
    expect(store.propertiesTaskId).toBeNull();
    expect(store.propertiesTask).toBeNull();

    const t = makeTask({ id: 'prop-1', filename: 'movie.mp4' });
    store.tasks = [t];

    store.openPropertiesModal('prop-1');
    expect(store.isPropertiesModalOpen).toBe(true);
    expect(store.propertiesTaskId).toBe('prop-1');
    expect(store.propertiesTask?.filename).toBe('movie.mp4');

    store.closePropertiesModal();
    expect(store.isPropertiesModalOpen).toBe(false);
    expect(store.propertiesTaskId).toBeNull();
    expect(store.propertiesTask).toBeNull();
  });

  it('handles moveTaskFile and getRecentLogs successfully and with errors', async () => {
    const store = new IdmStore();
    const t = makeTask({ id: 'task-move-1', file_path: 'C:\\old\\file.mp4' });
    store.tasks = [t];

    const updated = { ...t, file_path: 'D:\\new\\file.mp4', save_dir: 'D:\\new' };
    mockInvoke.mockResolvedValueOnce(updated);

    await store.moveTaskFile('task-move-1', 'D:\\new');
    expect(mockInvoke).toHaveBeenCalledWith('move_downloaded_file', {
      taskId: 'task-move-1',
      newDir: 'D:\\new',
    });
    expect(store.tasks[0].file_path).toBe('D:\\new\\file.mp4');

    // Move task not present in local array
    mockInvoke.mockResolvedValueOnce({ ...t, id: 'unlisted-task' });
    await store.moveTaskFile('unlisted-task', 'E:\\other');

    // Error case
    mockInvoke.mockRejectedValueOnce(new Error('OS move error'));
    await expect(store.moveTaskFile('task-move-1', 'Z:\\invalid')).rejects.toThrow('OS move error');

    // getRecentLogs
    mockInvoke.mockResolvedValueOnce(['[INFO] Log 1', '[INFO] Log 2']);
    const logs = await store.getRecentLogs(50);
    expect(mockInvoke).toHaveBeenCalledWith('get_recent_logs', { maxLines: 50 });
    expect(logs).toEqual(['[INFO] Log 1', '[INFO] Log 2']);

    // getRecentLogs error fallback
    mockInvoke.mockRejectedValueOnce(new Error('Log read failure'));
    const emptyLogs = await store.getRecentLogs();
    expect(emptyLogs).toEqual([]);

    // Non-Tauri fallback branch
    const savedWindow = (globalThis as any).window;
    delete (globalThis as any).window;
    await store.moveTaskFile('dummy', 'dummy');
    const previewLogs = await store.getRecentLogs();
    expect(previewLogs[0]).toContain('[Preview Mode]');
    (globalThis as any).window = savedWindow;
  });

  it('handles failed event when task already has error object', async () => {
    const store = new IdmStore();
    await store.setupEventListeners();
    const failedHandler = eventListeners.get('download-failed');
    expect(failedHandler).toBeDefined();

    store.tasks = [makeTask({ id: 't-prefailed', status: { failed: 'Server dropped stream' } })];
    failedHandler!({ payload: 't-prefailed' });
    expect(store.outcomeErrorMessage).toBe('Server dropped stream');
  });

  it('handles startRefreshLink and cancelRefreshLink', () => {
    const store = new IdmStore();
    const task = makeTask({
      id: 'refresh-1',
      url: 'https://example.com/file.zip',
      referer: 'https://example.com/page',
    });
    store.tasks = [task];
    store.isOutcomeModalOpen = true;
    store.isPropertiesModalOpen = true;
    store.isProgressModalOpen = true;

    const openSpy = vi.spyOn(store, 'openExternalUrl').mockImplementation(async () => { });

    // Task not found
    store.startRefreshLink('non-existent');
    expect(store.isRefreshModalOpen).toBe(false);

    // Task found
    store.startRefreshLink('refresh-1');
    expect(store.isRefreshModalOpen).toBe(true);
    expect(store.refreshTaskId).toBe('refresh-1');
    expect(store.refreshDetectedUrl).toBeNull();
    expect(store.refreshError).toBeNull();
    expect(store.refreshCountdown).toBe(3);
    expect(store.refreshTask?.id).toBe('refresh-1');
    expect(store.isOutcomeModalOpen).toBe(false);
    expect(store.isPropertiesModalOpen).toBe(false);
    expect(store.isProgressModalOpen).toBe(false);
    expect(openSpy).toHaveBeenCalledWith('https://example.com/page');

    // cancel
    store.cancelRefreshLink();
    expect(store.isRefreshModalOpen).toBe(false);
    expect(store.refreshTaskId).toBeNull();
    expect(store.refreshDetectedUrl).toBeNull();
    expect(store.refreshError).toBeNull();
  });

  it('routes browser-download-requested to refreshDetectedUrl when refresh modal is active', async () => {
    const store = new IdmStore();
    await store.setupEventListeners();
    const handler = eventListeners.get('browser-download-requested');
    expect(handler).toBeDefined();

    store.isRefreshModalOpen = true;
    store.refreshTaskId = 'refresh-active';

    handler!({
      payload: {
        url: 'https://cdn.example.com/refreshed-link.mp4',
        filename: 'video.mp4',
        headers: {},
      },
    });

    expect(store.refreshDetectedUrl).toBe('https://cdn.example.com/refreshed-link.mp4');
    expect(store.isAddModalOpen).toBe(false);
  });

  it('applies refreshed URL successfully', async () => {
    const store = new IdmStore();
    const task = makeTask({
      id: 'task-refresh-apply',
      url: 'https://old.com/expired.iso',
      status: { failed: 'Expired token' } as any,
    });
    store.tasks = [task];
    store.refreshTaskId = 'task-refresh-apply';
    store.isRefreshModalOpen = true;

    mockInvoke.mockResolvedValue(undefined);
    const resumeSpy = vi.spyOn(store, 'resumeTask').mockResolvedValue();

    await store.applyRefreshedUrl('https://new.com/fresh.iso');

    expect(mockInvoke).toHaveBeenCalledWith('refresh_download_url', {
      taskId: 'task-refresh-apply',
      newUrl: 'https://new.com/fresh.iso',
    });
    expect(store.tasks[0].url).toBe('https://new.com/fresh.iso');
    expect(store.tasks[0].error_message).toBeNull();
    expect(store.tasks[0].status).toBe('paused');
    expect(store.isRefreshModalOpen).toBe(false);
    expect(resumeSpy).toHaveBeenCalledWith('task-refresh-apply');

    // Early return if no refreshTaskId
    store.refreshTaskId = null;
    await store.applyRefreshedUrl('https://another.com/file');
  });

  it('handles error in applyRefreshedUrl', async () => {
    const store = new IdmStore();
    store.tasks = [makeTask({ id: 'task-refresh-err' })];
    store.refreshTaskId = 'task-refresh-err';

    mockInvoke.mockRejectedValueOnce(new Error('URL size mismatch'));

    await expect(store.applyRefreshedUrl('https://bad.com/file')).rejects.toThrow(
      'URL size mismatch'
    );
    expect(store.refreshError).toBe('URL size mismatch');
  });

  it('handles openExternalUrl in Tauri and browser fallback', async () => {
    const store = new IdmStore();
    mockInvoke.mockResolvedValue(undefined);

    await store.openExternalUrl('https://google.com');
    expect(mockInvoke).toHaveBeenCalledWith('open_external_url', { url: 'https://google.com' });

    // IPC error branch
    mockInvoke.mockRejectedValueOnce(new Error('Shell error'));
    await store.openExternalUrl('https://error.com'); // should not throw

    // Non-Tauri fallback branch
    mockIsTauriReturn = false;
    const windowOpenMock = vi.fn();
    (globalThis as any).window = { open: windowOpenMock };

    await store.openExternalUrl('https://browser.com');
    expect(windowOpenMock).toHaveBeenCalledWith('https://browser.com', '_blank');

    mockIsTauriReturn = true;
  });

  it('handles setGlobalSpeedLimit and derived globalSpeedLimitBps', async () => {
    const store = new IdmStore();
    expect(store.speedLimiterEnabled).toBe(false);
    expect(store.globalSpeedLimitBps).toBeNull();

    // Enable with 2 MB/s
    await store.setGlobalSpeedLimit(true, 2, 'MB/s');
    expect(store.speedLimiterEnabled).toBe(true);
    expect(store.globalSpeedLimitValue).toBe(2);
    expect(store.globalSpeedLimitUnit).toBe('MB/s');
    expect(store.globalSpeedLimitBps).toBe(2097152);
    expect(mockInvoke).toHaveBeenCalledWith('set_global_speed_limit', {
      enabled: true,
      limitBps: 2097152,
    });

    // Change to 500 KB/s
    await store.setGlobalSpeedLimit(true, 500, 'KB/s');
    expect(store.globalSpeedLimitValue).toBe(500);
    expect(store.globalSpeedLimitUnit).toBe('KB/s');
    expect(store.globalSpeedLimitBps).toBe(512000);
    expect(mockInvoke).toHaveBeenCalledWith('set_global_speed_limit', {
      enabled: true,
      limitBps: 512000,
    });

    // Disable limit
    await store.setGlobalSpeedLimit(false);
    expect(store.speedLimiterEnabled).toBe(false);
    expect(store.globalSpeedLimitBps).toBeNull();
    expect(mockInvoke).toHaveBeenCalledWith('set_global_speed_limit', {
      enabled: false,
      limitBps: null,
    });

    // IPC error branch
    mockInvoke.mockRejectedValueOnce(new Error('IPC failed'));
    await store.setGlobalSpeedLimit(true, 1, 'MB/s'); // shouldn't throw
  });

  it('handles setTaskSpeedLimit', async () => {
    const store = new IdmStore();
    const task = makeTask({ id: 'task-speed-1', speed_limit_bps: null });
    store.tasks = [task];

    mockInvoke.mockResolvedValue(undefined);
    await store.setTaskSpeedLimit('task-speed-1', 1048576);

    expect(store.tasks[0].speed_limit_bps).toBe(1048576);
    expect(mockInvoke).toHaveBeenCalledWith('set_task_speed_limit', {
      taskId: 'task-speed-1',
      limitBps: 1048576,
    });

    // Clear limit
    await store.setTaskSpeedLimit('task-speed-1', null);
    expect(store.tasks[0].speed_limit_bps).toBeNull();

    // IPC error branch
    mockInvoke.mockRejectedValueOnce(new Error('DB error'));
    await store.setTaskSpeedLimit('task-speed-1', 512000); // shouldn't throw
  });

  it('initializes store and loads appVersion on init()', async () => {
    mockIsTauriReturn = true;
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_all_tasks') return Promise.resolve([]);
      if (cmd === 'get_global_speed_limit') return Promise.resolve({ enabled: false, limit_bps: null });
      return Promise.resolve(undefined);
    });

    const store = new IdmStore();
    await store.init();
    expect(store.appVersion).toBeDefined();
    expect(store.appVersion).toBe('0.1.0-dev');
  });

  it('handles settings lifecycle: load, apply, save, reset, and localStorage fallback', async () => {
    mockIsTauriReturn = true;
    mockInvoke.mockImplementation((cmd: string, args?: any) => {
      if (cmd === 'get_app_settings') {
        return Promise.resolve({
          autoStartWindows: 'false',
          defaultConnections: '32',
          connectionType: 'broadband',
          defaultDownloadDir: 'D:\\Downloads',
        });
      }
      if (cmd === 'save_app_settings') {
        return Promise.resolve();
      }
      if (cmd === 'get_default_download_dir') {
        return Promise.resolve('C:\\Users\\Test\\Downloads');
      }
      return Promise.resolve(undefined);
    });

    const store = new IdmStore();
    await store.loadAppSettings();

    expect(store.settings.autoStartWindows).toBe(false);
    expect(store.settings.defaultConnections).toBe(32);
    expect(store.settings.defaultDownloadDir).toBe('D:\\Downloads');

    // Save partial update
    await store.saveAppSettings({ defaultConnections: 24, autoStartWindows: true });
    expect(store.settings.defaultConnections).toBe(24);
    expect(store.settings.autoStartWindows).toBe(true);
    expect(mockInvoke).toHaveBeenCalledWith('save_app_settings', {
      settings: expect.objectContaining({
        defaultConnections: '24',
        autoStartWindows: 'true',
      }),
    });

    // Reset settings
    await store.resetAppSettings();
    expect(store.settings.defaultConnections).toBe(16);

    // Browser mode fallback with localStorage
    mockIsTauriReturn = false;
    const localStorageMock: Record<string, string> = {};
    (globalThis as any).window = {
      localStorage: {
        getItem: (k: string) => localStorageMock[k] || null,
        setItem: (k: string, v: string) => {
          localStorageMock[k] = v;
        },
      },
    };

    const webStore = new IdmStore();
    await webStore.saveAppSettings({ maxRetries: 10 });
    expect(localStorageMock['myownidm_settings']).toBeDefined();

    const restoredStore = new IdmStore();
    await restoredStore.loadAppSettings();
    expect(restoredStore.settings.maxRetries).toBe(10);
  });

  it('handles clipboard monitoring selectively', async () => {
    const store = new IdmStore();
    store.settings.clipboardAutoCapture = true;
    store.isAddModalOpen = false;

    let clipboardText = 'https://example.com/software.zip';
    const mockClipboard = {
      readText: vi.fn().mockImplementation(() => Promise.resolve(clipboardText)),
    };
    try {
      Object.defineProperty(globalThis, 'navigator', {
        value: { clipboard: mockClipboard },
        configurable: true,
        writable: true,
      });
    } catch {
      Object.defineProperty(globalThis.navigator, 'clipboard', {
        value: mockClipboard,
        configurable: true,
        writable: true,
      });
    }

    // First check should detect zip file and open add modal
    await store.checkClipboardForUrl();
    expect(store.isAddModalOpen).toBe(true);
    expect(store.initialAddUrl).toBe('https://example.com/software.zip');

    // Close modal and re-check same clipboard text: should not duplicate
    store.isAddModalOpen = false;
    await store.checkClipboardForUrl();
    expect(store.isAddModalOpen).toBe(false);

    // Unrelated text or non-download url should be ignored
    clipboardText = 'Hello World this is just normal text';
    await store.checkClipboardForUrl();
    expect(store.isAddModalOpen).toBe(false);

    // Disabled setting should not read clipboard
    store.settings.clipboardAutoCapture = false;
    clipboardText = 'https://example.com/another.mp4';
    await store.checkClipboardForUrl();
    expect(store.isAddModalOpen).toBe(false);
  });

  it('triggers OS notification on download-completed when enabled', async () => {
    const notificationMock = vi.fn();
    (globalThis as any).Notification = Object.assign(notificationMock, {
      permission: 'granted',
    });

    const store = new IdmStore();
    store.settings.notifyOnComplete = true;
    store.tasks = [makeTask({ id: 'task-notif-1', filename: 'clip.mp4' })];

    await store.setupEventListeners();
    const completedListener = eventListeners.get('download-completed');
    expect(completedListener).toBeDefined();

    completedListener!({ payload: 'task-notif-1' });
    expect(notificationMock).toHaveBeenCalledWith('Unduhan Selesai - IDM Turbo', {
      body: 'clip.mp4',
      icon: '/favicon.png',
    });
  });

  it('handles checkDuplicateDownload in Tauri and browser fallback modes', async () => {
    const store = new IdmStore();

    // 1. Empty URL
    const resEmpty = await store.checkDuplicateDownload('');
    expect(resEmpty.is_duplicate).toBe(false);

    // 2. Tauri invoke mode
    mockIsTauriReturn = true;
    const mockDupResult = {
      is_duplicate: true,
      status: 'downloading',
      task_id: 'task-dup-1',
      filename: 'file.iso',
      file_path: 'C:\\Downloads\\file.iso',
      file_exists_on_disk: true,
      downloaded_bytes: 500,
      total_bytes: 1000,
      percent: 50.0,
      completed_at: null,
      suggested_new_filename: 'file (1).iso',
    };
    mockInvoke.mockResolvedValueOnce(mockDupResult);

    const resTauri = await store.checkDuplicateDownload('https://example.com/file.iso', 'file.iso', 'C:\\Downloads');
    expect(resTauri.is_duplicate).toBe(true);
    expect(resTauri.suggested_new_filename).toBe('file (1).iso');
    expect(mockInvoke).toHaveBeenCalledWith('check_duplicate_download', {
      url: 'https://example.com/file.iso',
      filename: 'file.iso',
      saveDir: 'C:\\Downloads',
    });

    // 3. Browser fallback mode (isTauri = false)
    (globalThis as any).window = {};
    mockIsTauriReturn = false;
    store.tasks = [
      makeTask({
        id: 'task-web-1',
        url: 'https://example.com/web-file.zip',
        filename: 'web-file.zip',
        file_path: 'D:\\web-file.zip',
        total_bytes: 2000,
        downloaded_bytes: 1000,
        status: 'downloading',
      }),
    ];

    const resBrowserMatched = await store.checkDuplicateDownload('https://example.com/web-file.zip');
    expect(resBrowserMatched.is_duplicate).toBe(true);
    expect(resBrowserMatched.suggested_new_filename).toBe('web-file (1).zip');
    expect(resBrowserMatched.percent).toBe(50);

    const resBrowserNotMatched = await store.checkDuplicateDownload('https://example.com/other-new.zip');
    expect(resBrowserNotMatched.is_duplicate).toBe(false);
  });

  it('handles openDuplicateModal, closeDuplicateModal, and proceedWithNewDownload', () => {
    const store = new IdmStore();
    const mockDup: DuplicateCheckResult = {
      is_duplicate: true,
      status: 'downloading',
      task_id: 'task-1',
      filename: 'sample.zip',
      file_path: 'C:\\Downloads\\sample.zip',
      file_exists_on_disk: true,
      downloaded_bytes: 100,
      total_bytes: 200,
      percent: 50.0,
      completed_at: null,
      suggested_new_filename: 'sample (1).zip',
    };

    store.openDuplicateModal(mockDup, 'https://example.com/sample.zip', { 'User-Agent': 'Custom' });
    expect(store.isDuplicateModalOpen).toBe(true);
    expect(store.duplicateModalData).toEqual(mockDup);
    expect(store.duplicateModalUrl).toBe('https://example.com/sample.zip');
    expect(store.duplicateModalHeaders).toEqual({ 'User-Agent': 'Custom' });
    expect(store.isAddModalOpen).toBe(false);

    // Test proceedWithNewDownload
    store.proceedWithNewDownload();
    expect(store.isDuplicateModalOpen).toBe(false);
    expect(store.isAddModalOpen).toBe(true);
    expect(store.initialAddUrl).toBe('https://example.com/sample.zip');
    expect(store.initialFilename).toBe('sample (1).zip');
    expect(store.initialHeaders).toEqual({ 'User-Agent': 'Custom' });

    // Test closeDuplicateModal
    store.openDuplicateModal(mockDup, 'https://example.com/sample.zip');
    expect(store.isDuplicateModalOpen).toBe(true);
    store.closeDuplicateModal();
    expect(store.isDuplicateModalOpen).toBe(false);
    expect(store.duplicateModalData).toBeNull();
    expect(store.duplicateModalUrl).toBe('');
    expect(store.duplicateModalHeaders).toBeNull();
  });

  it('handles browser-download-requested event with duplicate detection', async () => {
    const store = new IdmStore();
    mockIsTauriReturn = true;

    await store.setupEventListeners();
    const browserListener = eventListeners.get('browser-download-requested');
    expect(browserListener).toBeDefined();

    // 1. When duplicate is found
    mockInvoke.mockResolvedValueOnce({
      is_duplicate: true,
      status: 'completed',
      task_id: 'task-dup-event',
      filename: 'document.pdf',
      file_path: 'C:\\Downloads\\document.pdf',
      file_exists_on_disk: true,
      downloaded_bytes: 1000,
      total_bytes: 1000,
      percent: 100.0,
      completed_at: '2026-09-21',
      suggested_new_filename: 'document (1).pdf',
    });

    await browserListener!({
      payload: {
        url: 'https://example.com/document.pdf',
        filename: 'document.pdf',
        headers: { Cookie: 'auth=1' },
      },
    });

    expect(store.isDuplicateModalOpen).toBe(true);
    expect(store.isAddModalOpen).toBe(false);
    expect(store.duplicateModalData?.status).toBe('completed');

    // 2. When not a duplicate
    store.closeDuplicateModal();
    mockInvoke.mockResolvedValueOnce({
      is_duplicate: false,
      status: null,
      task_id: null,
      filename: null,
      file_path: null,
      file_exists_on_disk: false,
      downloaded_bytes: 0,
      total_bytes: 0,
      percent: 0,
      completed_at: null,
      suggested_new_filename: null,
    });

    await browserListener!({
      payload: {
        url: 'https://example.com/fresh-brand-new.pdf',
        filename: 'fresh-brand-new.pdf',
      },
    });

    expect(store.isDuplicateModalOpen).toBe(false);
    expect(store.isAddModalOpen).toBe(true);
    expect(store.initialAddUrl).toBe('https://example.com/fresh-brand-new.pdf');
  });

  describe('Task Sorting & Ordering', () => {
    it('sorts tasks by name asc and desc', () => {
      const store = new IdmStore();
      store.tasks = [
        makeTask({ id: '1', filename: 'Charlie.mp4' }),
        makeTask({ id: '2', filename: 'Alpha.mp4' }),
        makeTask({ id: '3', filename: 'Bravo.mp4' }),
      ];

      store.setSort('name', 'asc');
      expect(store.filteredTasks.map((t) => t.filename)).toEqual([
        'Alpha.mp4',
        'Bravo.mp4',
        'Charlie.mp4',
      ]);

      store.setSort('name', 'desc');
      expect(store.filteredTasks.map((t) => t.filename)).toEqual([
        'Charlie.mp4',
        'Bravo.mp4',
        'Alpha.mp4',
      ]);
    });

    it('sorts tasks by size asc and desc', () => {
      const store = new IdmStore();
      store.tasks = [
        makeTask({ id: '1', filename: 'medium.bin', total_bytes: 500, downloaded_bytes: 500 }),
        makeTask({ id: '2', filename: 'small.bin', total_bytes: 100, downloaded_bytes: 100 }),
        makeTask({ id: '3', filename: 'large.bin', total_bytes: 2000, downloaded_bytes: 2000 }),
      ];

      store.setSort('size', 'asc');
      expect(store.filteredTasks.map((t) => t.filename)).toEqual([
        'small.bin',
        'medium.bin',
        'large.bin',
      ]);

      store.setSort('size', 'desc');
      expect(store.filteredTasks.map((t) => t.filename)).toEqual([
        'large.bin',
        'medium.bin',
        'small.bin',
      ]);
    });

    it('sorts tasks by date asc and desc', () => {
      const store = new IdmStore();
      store.tasks = [
        makeTask({ id: '1', filename: 'old.bin', created_at: '2026-09-01 10:00:00' }),
        makeTask({ id: '2', filename: 'newest.bin', created_at: '2026-09-21 12:00:00' }),
        makeTask({ id: '3', filename: 'mid.bin', created_at: '2026-09-10 11:00:00' }),
      ];

      store.setSort('date', 'asc');
      expect(store.filteredTasks.map((t) => t.filename)).toEqual([
        'old.bin',
        'mid.bin',
        'newest.bin',
      ]);

      store.setSort('date', 'desc');
      expect(store.filteredTasks.map((t) => t.filename)).toEqual([
        'newest.bin',
        'mid.bin',
        'old.bin',
      ]);
    });

    it('sorts tasks by progress asc and desc', () => {
      const store = new IdmStore();
      store.tasks = [
        makeTask({ id: '1', filename: 'half.bin', total_bytes: 1000, downloaded_bytes: 500, status: 'downloading' }),
        makeTask({ id: '2', filename: 'done.bin', total_bytes: 1000, downloaded_bytes: 1000, status: 'completed' }),
        makeTask({ id: '3', filename: 'zero.bin', total_bytes: 1000, downloaded_bytes: 0, status: 'queued' }),
      ];

      store.setSort('progress', 'asc');
      expect(store.filteredTasks.map((t) => t.filename)).toEqual([
        'zero.bin',
        'half.bin',
        'done.bin',
      ]);

      store.setSort('progress', 'desc');
      expect(store.filteredTasks.map((t) => t.filename)).toEqual([
        'done.bin',
        'half.bin',
        'zero.bin',
      ]);
    });

    it('sorts tasks by speed asc and desc', () => {
      const store = new IdmStore();
      store.tasks = [
        makeTask({ id: '1', filename: 'medium.bin', speed_bps: 200000 }),
        makeTask({ id: '2', filename: 'fast.bin', speed_bps: 1000000 }),
        makeTask({ id: '3', filename: 'slow.bin', speed_bps: 50000 }),
      ];

      store.setSort('speed', 'asc');
      expect(store.filteredTasks.map((t) => t.filename)).toEqual([
        'slow.bin',
        'medium.bin',
        'fast.bin',
      ]);

      store.setSort('speed', 'desc');
      expect(store.filteredTasks.map((t) => t.filename)).toEqual([
        'fast.bin',
        'medium.bin',
        'slow.bin',
      ]);
    });

    it('sorts tasks by status asc and desc', () => {
      const store = new IdmStore();
      store.tasks = [
        makeTask({ id: '1', filename: 'task1.bin', status: 'paused' }),
        makeTask({ id: '2', filename: 'task2.bin', status: 'completed' }),
        makeTask({ id: '3', filename: 'task3.bin', status: 'downloading' }),
      ];

      store.setSort('status', 'asc');
      expect(store.filteredTasks.map((t) => t.filename)).toEqual([
        'task2.bin', // 'completed'
        'task3.bin', // 'downloading'
        'task1.bin', // 'paused'
      ]);

      store.setSort('status', 'desc');
      expect(store.filteredTasks.map((t) => t.filename)).toEqual([
        'task1.bin', // 'paused'
        'task3.bin', // 'downloading'
        'task2.bin', // 'completed'
      ]);
    });

    it('toggles sort order and cycles criteria correctly', () => {
      const store = new IdmStore();
      expect(store.sortBy).toBe('date');
      expect(store.sortOrder).toBe('desc');

      // Toggling order
      store.toggleSortOrder();
      expect(store.sortOrder).toBe('asc');
      store.toggleSortOrder();
      expect(store.sortOrder).toBe('desc');

      // Clicking same criterion without order toggles order
      store.setSort('date');
      expect(store.sortOrder).toBe('asc');
      store.setSort('date');
      expect(store.sortOrder).toBe('desc');

      // Clicking new criterion sets sensible default
      store.setSort('name');
      expect(store.sortBy).toBe('name');
      expect(store.sortOrder).toBe('asc');

      store.setSort('size');
      expect(store.sortBy).toBe('size');
      expect(store.sortOrder).toBe('desc');

      // Cycling criteria
      store.sortBy = 'date';
      store.cycleSortCriteria();
      expect(store.sortBy).toBe('size');
      store.cycleSortCriteria();
      expect(store.sortBy).toBe('name');
      store.cycleSortCriteria();
      expect(store.sortBy).toBe('progress');
      store.cycleSortCriteria();
      expect(store.sortBy).toBe('speed');
      store.cycleSortCriteria();
      expect(store.sortBy).toBe('status');
      store.cycleSortCriteria();
      expect(store.sortBy).toBe('date');
    });
  });

  describe('Language Settings & Translation', () => {
    it('initializes with default language id', () => {
      const store = new IdmStore();
      expect(store.settings.language).toBe('id');
      expect(store.t('common.save')).toBe('Simpan');
    });

    it('translates reactively when language changes', async () => {
      const store = new IdmStore();
      expect(store.t('common.cancel')).toBe('Batal');

      await store.setLanguage('en');
      expect(store.settings.language).toBe('en');
      expect(store.t('common.cancel')).toBe('Cancel');
      expect(store.t('toolbar.addUrl')).toBe('Add URL');

      await store.setLanguage('id');
      expect(store.settings.language).toBe('id');
      expect(store.t('common.cancel')).toBe('Batal');
      expect(store.t('toolbar.addUrl')).toBe('Tambah URL');
    });

    it('persists and loads language in raw settings conversion', () => {
      const store = new IdmStore();
      const raw = store.settingsToRaw({ ...store.settings, language: 'en' });
      expect(raw.language).toBe('en');

      store.applyRawSettings({ language: 'en' });
      expect(store.settings.language).toBe('en');

      store.applyRawSettings({ language: 'id' });
      expect(store.settings.language).toBe('id');

      // Invalid language in raw settings is ignored
      store.applyRawSettings({ language: 'invalid' });
      expect(store.settings.language).toBe('id');
    });
  });

  describe('Diagnostic & Error Reporting', () => {
    it('reports diagnostic error via Tauri invoke', async () => {
      mockIsTauriReturn = true;
      mockInvoke.mockImplementation((cmd, args) => {
        if (cmd === 'report_diagnostic_error') {
          return Promise.resolve('sentry-evt-12345');
        }
        return Promise.resolve(undefined);
      });

      const store = new IdmStore();
      const id = await store.reportDiagnostic('task-abc-123', 'HTTP 403 Forbidden');
      expect(id).toBe('sentry-evt-12345');
      expect(mockInvoke).toHaveBeenCalledWith('report_diagnostic_error', {
        taskId: 'task-abc-123',
        errorMessage: 'HTTP 403 Forbidden',
      });
    });

    it('falls back to local mock ID in non-Tauri browser environment', async () => {
      (globalThis as any).window = {};
      mockIsTauriReturn = false;
      const store = new IdmStore();
      const id = await store.reportDiagnostic('task-abc-123', 'Network timeout');
      expect(id).toBe('local-diag-task-abc');
    });


    it('propagates error if Tauri invoke fails', async () => {
      mockIsTauriReturn = true;
      mockInvoke.mockImplementation((cmd) => {
        if (cmd === 'report_diagnostic_error') {
          return Promise.reject(new Error('Network error'));
        }
        return Promise.resolve(undefined);
      });

      const store = new IdmStore();
      await expect(store.reportDiagnostic('task-abc-123')).rejects.toThrow('Network error');
    });
  });
});





