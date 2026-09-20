<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { store } from '$lib/idmStore.svelte';
  import {
    formatBytes,
    formatEta,
    formatSpeed,
    type DownloadTask,
    type SpeedLimitUnit,
    bpsToUnit,
    unitToBps
  } from '$lib/types';
  import {
    Download,
    X,
    Minus,
    Play,
    Pause,
    CheckCircle2,
    Sliders,
    Settings,
    Gauge,
    Check,
    Folder,
    FolderOpen,
    FileText,
    Video,
    Music,
    Archive,
    Cpu,
    AlertTriangle,
    RotateCcw,
    RotateCw,
    Copy,
    ChevronDown,
    ChevronUp,
    ExternalLink
  } from '@lucide/svelte';

  const taskId = $derived(page.url.searchParams.get('id') || '');
  const task = $derived(store.tasks.find((t) => t.id === taskId) || null);

  let activeTab = $state<'status' | 'limiter' | 'options'>('status');
  let showDetails = $state<boolean>(true);
  let taskLimiterEnabled = $state<boolean>(false);
  let taskLimitValue = $state<number>(1);
  let taskLimitUnit = $state<SpeedLimitUnit>('MB/s');
  let copied = $state<boolean>(false);
  let autoOpenOnClose = $state<boolean>(false);

  import { invoke } from '@tauri-apps/api/core';

  onMount(async () => {
    await store.init();
  });

  $effect(() => {
    if (task) {
      if (task.speed_limit_bps && task.speed_limit_bps > 0) {
        taskLimiterEnabled = true;
        const parsed = bpsToUnit(task.speed_limit_bps);
        taskLimitValue = parsed.value;
        taskLimitUnit = parsed.unit;
      } else {
        taskLimiterEnabled = false;
      }
    }
  });

  async function applyTaskLimit() {
    if (!task) return;
    const bps = taskLimiterEnabled ? unitToBps(taskLimitValue, taskLimitUnit) : null;
    await store.setTaskSpeedLimit(task.id, bps);
  }

  function getPercent(t: DownloadTask): number {
    if (t.status === 'completed') return 100;
    if (!t.total_bytes || t.total_bytes === 0) return 0;
    return Math.min(100, Math.max(0, (t.downloaded_bytes / t.total_bytes) * 100));
  }

  const pct = $derived(task ? getPercent(task) : 0);
  const isDownloading = $derived(task?.status === 'downloading');
  const isCompleted = $derived(task?.status === 'completed');
  const isPaused = $derived(task?.status === 'paused');
  const isFailed = $derived(!!task && typeof task.status === 'object' && 'failed' in task.status);
  const errorMessage = $derived(
    task && typeof task.status === 'object' && 'failed' in task.status
      ? (task.status as { failed: string }).failed
      : task?.error_message || 'Koneksi terputus atau URL kadaluarsa (HTTP 403 / Timeout)'
  );

  function getFileIcon(cat?: string) {
    switch (cat) {
      case 'video':
        return Video;
      case 'audio':
        return Music;
      case 'compressed':
        return Archive;
      case 'programs':
        return Cpu;
      case 'documents':
        return FileText;
      default:
        return Folder;
    }
  }

  const FileIcon = $derived(getFileIcon(task?.category));

  async function minimizeWindow(e?: MouseEvent) {
    e?.stopPropagation();
    e?.preventDefault();
    try {
      await invoke('minimize_current_window');
    } catch {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        await getCurrentWindow().minimize();
      } catch (err) {
        console.warn('Minimize fallback failed:', err);
      }
    }
  }

  async function closeWindow(e?: MouseEvent) {
    e?.stopPropagation();
    e?.preventDefault();
    try {
      await invoke('close_current_window');
    } catch {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        await getCurrentWindow().close();
      } catch {
        if (typeof window !== 'undefined') {
          window.close();
        }
      }
    }
  }

  async function handleHeaderMouseDown(e: MouseEvent) {
    if ((e.target as HTMLElement).closest('button, a, input, select')) return;
    try {
      await invoke('start_dragging_window');
    } catch {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        await getCurrentWindow().startDragging();
      } catch (err) {
        console.warn('Start dragging failed:', err);
      }
    }
  }

  async function handleCopyPath() {
    if (!task?.file_path) return;
    try {
      await navigator.clipboard.writeText(task.file_path);
      copied = true;
      setTimeout(() => {
        copied = false;
      }, 2000);
    } catch (e) {
      console.error('Clipboard copy failed:', e);
    }
  }

  async function handleOpenFile() {
    if (!task?.file_path) return;
    await store.openFile(task.file_path);
    await closeWindow();
  }

  async function handleOpenFolder() {
    if (!task?.file_path) return;
    await store.openFolder(task.file_path);
  }

  async function handleRetry() {
    if (!task?.id) return;
    await store.resumeTask(task.id);
  }

  function handleRefreshLink() {
    if (!task) return;
    store.startRefreshLink(task.id);
  }
</script>

<main class="w-full h-screen bg-[#090e16] text-slate-100 flex flex-col select-none overflow-hidden font-sans border border-[#252a33] box-border">
  {#if !task}
    <!-- Loading or Not Found State -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <header
      data-tauri-drag-region
      onmousedown={handleHeaderMouseDown}
      class="h-10 px-3 bg-[#0f141c] border-b border-[#252a33] flex items-center justify-between cursor-move shrink-0"
    >
      <span class="text-xs font-medium text-slate-400">Transfer Unduhan</span>
      <button
        type="button"
        onclick={closeWindow}
        class="w-7 h-7 flex items-center justify-center rounded text-slate-400 hover:text-white hover:bg-red-600/80 transition-colors cursor-pointer"
        title="Tutup"
      >
        <X class="w-3.5 h-3.5 pointer-events-none" />
      </button>
    </header>
    <div class="flex-1 flex flex-col items-center justify-center p-6 text-center">
      <div class="w-10 h-10 rounded-full bg-[#171c24] flex items-center justify-center text-[#00e5ff] mb-3 animate-spin">
        <RotateCw class="w-5 h-5" />
      </div>
      <p class="text-sm text-slate-300 font-medium">Memuat rincian unduhan...</p>
      <p class="text-xs text-slate-500 mt-1 font-mono">{taskId}</p>
    </div>
  {:else}
    <!-- Top Accent Line -->
    <div
      class="h-[2px] w-full shrink-0 {isCompleted
        ? 'bg-gradient-to-r from-[#10b981] via-[#4cd7f6] to-[#10b981]'
        : isFailed
          ? 'bg-gradient-to-r from-[#ff5252] via-amber-400 to-[#ff5252]'
          : 'bg-gradient-to-r from-[#2563eb] via-[#00e5ff] to-[#4cd7f6]'}"
    ></div>

    <!-- Custom Window Titlebar -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <header
      data-tauri-drag-region
      onmousedown={handleHeaderMouseDown}
      class="h-10 px-3.5 bg-[#0f141c] border-b border-[#1f242e] flex items-center justify-between select-none cursor-move shrink-0"
    >
      <div class="flex items-center gap-2 overflow-hidden pr-2 pointer-events-none">
        <div
          class="flex items-center justify-center w-5 h-5 rounded-md shrink-0 {isCompleted
            ? 'bg-[#10b981]/20 text-[#10b981]'
            : isFailed
              ? 'bg-[#ff5252]/20 text-[#ff5252]'
              : 'bg-[#00e5ff]/20 text-[#00e5ff]'}"
        >
          {#if isCompleted}
            <CheckCircle2 class="w-3.5 h-3.5" />
          {:else if isFailed}
            <AlertTriangle class="w-3.5 h-3.5" />
          {:else}
            <img src="/favicon.png" alt="IDM Turbo" class="w-4 h-4 object-contain" />
          {/if}
        </div>
        <h1 class="text-xs font-semibold text-slate-200 tracking-tight truncate flex items-center gap-1.5">
          <span
            class="font-bold font-mono {isCompleted
              ? 'text-[#10b981]'
              : isFailed
                ? 'text-[#ff5252]'
                : 'text-[#00e5ff]'}"
          >
            {pct.toFixed(0)}%
          </span>
          <span class="truncate max-w-[340px]">{task.filename}</span>
        </h1>
      </div>

      <div class="flex items-center gap-1 shrink-0">
        <button
          onclick={minimizeWindow}
          class="w-7 h-7 flex items-center justify-center rounded text-slate-400 hover:text-slate-100 hover:bg-[#1a2236] transition-colors cursor-pointer"
          title="Minimalkan"
          type="button"
        >
          <Minus class="w-3.5 h-3.5 pointer-events-none" />
        </button>
        <button
          onclick={closeWindow}
          class="w-7 h-7 flex items-center justify-center rounded text-slate-400 hover:text-white hover:bg-red-600/80 transition-colors cursor-pointer"
          title="Tutup"
          type="button"
        >
          <X class="w-3.5 h-3.5 pointer-events-none" />
        </button>
      </div>
    </header>

    <!-- Content Switcher: In-Place Transitions -->
    {#if isCompleted}
      <!-- ================= PHASE: COMPLETED ================= -->
      <div class="flex-1 p-4 flex flex-col justify-between overflow-y-auto bg-[#090e16]">
        <div class="space-y-3.5">
          <!-- Success Banner -->
          <div class="flex items-center gap-3 p-3 rounded-xl bg-[#10b981]/10 border border-[#10b981]/30">
            <div class="w-10 h-10 rounded-lg bg-[#10b981]/20 flex items-center justify-center text-[#10b981] shrink-0 shadow-[0_0_15px_rgba(16,185,129,0.3)]">
              <CheckCircle2 class="w-6 h-6" />
            </div>
            <div class="min-w-0">
              <h2 class="text-sm font-bold text-white tracking-wide">Unduhan Selesai Sempurna</h2>
              <p class="text-xs text-slate-300 truncate font-mono">
                {formatBytes(task.downloaded_bytes)} • Berhasil disimpan ke disk
              </p>
            </div>
          </div>

          <!-- File Info Card -->
          <div class="bg-[#0f141c] p-3 rounded-xl border border-[#1f242e] space-y-2 text-xs">
            <div class="flex items-center justify-between">
              <span class="text-slate-400">Nama Berkas:</span>
              <span class="font-medium text-slate-200 truncate max-w-[320px]" title={task.filename}>
                {task.filename}
              </span>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-slate-400">Ukuran Akhir:</span>
              <span class="font-mono text-emerald-400 font-semibold">{formatBytes(task.downloaded_bytes)}</span>
            </div>
            <div class="flex items-start justify-between gap-2 pt-1 border-t border-[#1f242e]">
              <span class="text-slate-400 shrink-0">Lokasi:</span>
              <div class="flex items-center gap-1.5 min-w-0">
                <span class="font-mono text-[11px] text-[#4cd7f6] truncate max-w-[280px]" title={task.file_path}>
                  {task.file_path}
                </span>
                <button
                  type="button"
                  onclick={handleCopyPath}
                  class="p-1 rounded hover:bg-[#1f242e] text-slate-400 hover:text-white transition-colors shrink-0"
                  title="Salin lokasi berkas"
                >
                  {#if copied}
                    <Check class="w-3.5 h-3.5 text-emerald-400" />
                  {:else}
                    <Copy class="w-3.5 h-3.5" />
                  {/if}
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- Completion Actions -->
        <div class="pt-3 border-t border-[#1f242e] flex items-center justify-between gap-2">
          <button
            type="button"
            onclick={handleOpenFolder}
            class="px-3 py-1.5 text-xs font-semibold rounded-lg bg-[#171c24] hover:bg-[#252a33] text-slate-200 border border-[#252a33] flex items-center gap-1.5 transition-colors cursor-pointer"
          >
            <FolderOpen class="w-4 h-4 text-[#00e5ff]" />
            <span>Buka Folder</span>
          </button>

          <div class="flex items-center gap-2">
            <button
              type="button"
              onclick={closeWindow}
              class="px-3 py-1.5 text-xs font-semibold rounded-lg bg-[#171c24] hover:bg-[#252a33] text-slate-300 border border-[#252a33] transition-colors cursor-pointer"
            >
              Tutup
            </button>
            <button
              type="button"
              onclick={handleOpenFile}
              class="px-4 py-1.5 text-xs font-bold rounded-lg bg-[#10b981] hover:bg-[#10b981]/90 text-slate-950 flex items-center gap-1.5 shadow-[0_0_12px_rgba(16,185,129,0.4)] transition-all cursor-pointer"
            >
              <Play class="w-3.5 h-3.5 fill-current" />
              <span>Buka Berkas</span>
            </button>
          </div>
        </div>
      </div>
    {:else if isFailed}
      <!-- ================= PHASE: FAILED ================= -->
      <div class="flex-1 p-4 flex flex-col justify-between overflow-y-auto bg-[#090e16]">
        <div class="space-y-3.5">
          <!-- Error Banner -->
          <div class="flex items-center gap-3 p-3 rounded-xl bg-[#ff5252]/10 border border-[#ff5252]/30">
            <div class="w-10 h-10 rounded-lg bg-[#ff5252]/20 flex items-center justify-center text-[#ff5252] shrink-0 shadow-[0_0_15px_rgba(255,82,82,0.3)]">
              <AlertTriangle class="w-6 h-6" />
            </div>
            <div class="min-w-0">
              <h2 class="text-sm font-bold text-white tracking-wide">Unduhan Terputus atau Gagal</h2>
              <p class="text-xs text-[#ffb4ab] truncate font-mono">
                {errorMessage}
              </p>
            </div>
          </div>

          <!-- Diagnostic Details -->
          <div class="bg-[#0f141c] p-3 rounded-xl border border-[#1f242e] space-y-2 text-xs">
            <div class="flex items-center justify-between">
              <span class="text-slate-400">Telah Diunduh:</span>
              <span class="font-mono text-slate-200">{formatBytes(task.downloaded_bytes)} ({pct.toFixed(1)}%)</span>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-slate-400">Status Server:</span>
              <span class="text-amber-400 font-medium">Tautan mungkin perlu diperbarui dari web</span>
            </div>
            <div class="pt-1.5 border-t border-[#1f242e] text-[11px] text-slate-400">
              Progres unduhan parsial Anda tetap aman dan tidak akan hilang saat tautan diperbarui.
            </div>
          </div>
        </div>

        <!-- Failure Actions -->
        <div class="pt-3 border-t border-[#1f242e] flex items-center justify-between gap-2">
          <button
            type="button"
            onclick={handleRefreshLink}
            class="px-3.5 py-1.5 text-xs font-bold rounded-lg bg-amber-500/20 hover:bg-amber-500/30 text-amber-300 border border-amber-500/40 flex items-center gap-1.5 shadow-[0_0_10px_rgba(245,158,11,0.2)] transition-colors cursor-pointer"
          >
            <RotateCw class="w-3.5 h-3.5" />
            <span>Perbarui Tautan Unduhan...</span>
          </button>

          <div class="flex items-center gap-2">
            <button
              type="button"
              onclick={closeWindow}
              class="px-3 py-1.5 text-xs font-semibold rounded-lg bg-[#171c24] hover:bg-[#252a33] text-slate-300 border border-[#252a33] transition-colors cursor-pointer"
            >
              Tutup
            </button>
            <button
              type="button"
              onclick={handleRetry}
              class="px-4 py-1.5 text-xs font-bold rounded-lg bg-[#00e5ff] hover:bg-[#00e5ff]/90 text-slate-950 flex items-center gap-1.5 shadow-glow-cyan transition-all cursor-pointer"
            >
              <RotateCcw class="w-3.5 h-3.5" />
              <span>Coba Lagi</span>
            </button>
          </div>
        </div>
      </div>
    {:else}
      <!-- ================= PHASE: IN-FLIGHT (DOWNLOADING / PAUSED) ================= -->
      <div class="flex-1 flex flex-col justify-between overflow-y-auto bg-[#090e16]">
        <div>
          <!-- Tab Navigation -->
          <nav class="flex items-center bg-[#0b0f17] border-b border-[#1a2236] px-3 pt-1.5 select-none text-xs">
            <button
              onclick={() => (activeTab = 'status')}
              class="flex items-center gap-1.5 px-3 py-1.5 font-medium transition-all cursor-pointer {activeTab === 'status' ? 'text-[#00e5ff] border-b-2 border-[#00e5ff] bg-[#0f1420]/80 rounded-t-md font-semibold' : 'text-slate-400 hover:text-slate-200 border-b-2 border-transparent'}"
            >
              <Download class="w-3.5 h-3.5" />
              <span>Status Unduhan</span>
            </button>

            <button
              onclick={() => (activeTab = 'limiter')}
              class="flex items-center gap-1.5 px-3 py-1.5 font-medium transition-all cursor-pointer {activeTab === 'limiter' ? 'text-[#00e5ff] border-b-2 border-[#00e5ff] bg-[#0f1420]/80 rounded-t-md font-semibold' : 'text-slate-400 hover:text-slate-200 border-b-2 border-transparent'}"
            >
              <Gauge class="w-3.5 h-3.5" />
              <span>Pembatas Kecepatan</span>
            </button>

            <button
              onclick={() => (activeTab = 'options')}
              class="flex items-center gap-1.5 px-3 py-1.5 font-medium transition-all cursor-pointer {activeTab === 'options' ? 'text-[#00e5ff] border-b-2 border-[#00e5ff] bg-[#0f1420]/80 rounded-t-md font-semibold' : 'text-slate-400 hover:text-slate-200 border-b-2 border-transparent'}"
            >
              <Settings class="w-3.5 h-3.5" />
              <span>Opsi</span>
            </button>
          </nav>

          <!-- Tab Content Area -->
          <div class="p-3.5 space-y-3">
            {#if activeTab === 'status'}
              <section class="space-y-1.5 text-xs">
                <div class="bg-[#0f1420]/70 p-3 rounded-xl border border-[#1a2236] space-y-1.5">
                  <div class="flex items-center justify-between">
                    <span class="text-slate-400">Status:</span>
                    <span class="font-medium {isDownloading ? 'text-[#00e5ff]' : 'text-amber-400'} capitalize">
                      {typeof task.status === 'string' ? task.status : 'Menghubungkan'}
                    </span>
                  </div>

                  <div class="flex items-center justify-between">
                    <span class="text-slate-400">Ukuran Berkas:</span>
                    <span class="font-mono text-slate-200">{formatBytes(task.total_bytes)}</span>
                  </div>

                  <div class="flex items-center justify-between">
                    <span class="text-slate-400">Telah Diunduh:</span>
                    <span class="font-mono text-slate-200">
                      {formatBytes(task.downloaded_bytes)}
                      {#if task.total_bytes}
                        <span class="text-slate-400">({pct.toFixed(1)}%)</span>
                      {/if}
                    </span>
                  </div>

                  <div class="flex items-center justify-between">
                    <span class="text-slate-400">Kecepatan Transfer:</span>
                    <span class="font-mono font-bold text-emerald-400">
                      {isDownloading ? formatSpeed(task.speed_bps) : '0 B/s'}
                    </span>
                  </div>

                  <div class="flex items-center justify-between">
                    <span class="text-slate-400">Waktu Tersisa:</span>
                    <span class="font-mono font-medium text-amber-300">
                      {isDownloading ? `± ${formatEta(task.eta_seconds)}` : '--'}
                    </span>
                  </div>

                  <div class="flex items-center justify-between pt-1 border-t border-[#1a2236]/60">
                    <span class="text-slate-400">Dukungan Lanjutkan (Resume):</span>
                    <span class="inline-flex items-center gap-1 text-[#10b981] font-medium">
                      <CheckCircle2 class="w-3.5 h-3.5" />
                      <span>{task.supports_range ? 'Ya (Multi-Thread)' : 'Tidak (Single Stream)'}</span>
                    </span>
                  </div>
                </div>
              </section>
            {:else if activeTab === 'limiter'}
              <section class="space-y-2.5 text-xs">
                <div class="bg-[#0f1420]/70 p-3 rounded-xl border border-[#1a2236] space-y-3">
                  <div class="flex items-center justify-between">
                    <span class="text-slate-300 font-medium">Tingkat transfer saat ini:</span>
                    <span class="font-mono text-[#00e5ff] font-bold">{formatSpeed(task.speed_bps)}</span>
                  </div>

                  <label class="flex items-center gap-2 cursor-pointer select-none pt-1">
                    <input
                      type="checkbox"
                      bind:checked={taskLimiterEnabled}
                      onchange={applyTaskLimit}
                      class="w-4 h-4 rounded bg-[#0b0f17] border-[#222d45] text-[#00e5ff] focus:ring-0 cursor-pointer"
                    />
                    <span class="text-slate-200 font-medium">Batasi Kecepatan Khusus Unduhan Ini</span>
                  </label>

                  {#if taskLimiterEnabled}
                    <div class="space-y-2 pl-6 pt-1 border-t border-[#1a2236]/60">
                      <span class="block text-slate-400 text-[11px]">Batas kecepatan maksimum berkas ini:</span>
                      <div class="flex items-center gap-2">
                        <input
                          type="number"
                          min="1"
                          step="any"
                          bind:value={taskLimitValue}
                          oninput={applyTaskLimit}
                          class="w-24 px-2 py-1 font-mono bg-[#07090e] border border-[#222d45] rounded text-slate-100 focus:outline-none focus:border-[#00e5ff] text-xs"
                        />

                        <div class="flex rounded bg-[#07090e] p-0.5 border border-[#222d45]">
                          <button
                            type="button"
                            onclick={() => { taskLimitUnit = 'KB/s'; applyTaskLimit(); }}
                            class="px-2 py-0.5 text-[10px] font-mono font-medium rounded transition-colors {taskLimitUnit === 'KB/s' ? 'bg-[#00e5ff] text-slate-950 font-bold' : 'text-[#8c909f] hover:text-[#dee2ee]'}"
                          >
                            KB/s
                          </button>
                          <button
                            type="button"
                            onclick={() => { taskLimitUnit = 'MB/s'; applyTaskLimit(); }}
                            class="px-2 py-0.5 text-[10px] font-mono font-medium rounded transition-colors {taskLimitUnit === 'MB/s' ? 'bg-[#00e5ff] text-slate-950 font-bold' : 'text-[#8c909f] hover:text-[#dee2ee]'}"
                          >
                            MB/s
                          </button>
                        </div>
                      </div>

                      <div class="flex items-center gap-1.5 pt-1 flex-wrap">
                        <span class="text-[10px] text-slate-400">Pilihan Cepat:</span>
                        <button
                          type="button"
                          onclick={() => { taskLimitValue = 500; taskLimitUnit = 'KB/s'; applyTaskLimit(); }}
                          class="px-1.5 py-0.5 rounded bg-[#141b2b] hover:bg-[#1a2236] text-[10px] font-mono text-slate-300 hover:text-[#00e5ff] border border-[#222d45]"
                        >
                          500 KB/s
                        </button>
                        <button
                          type="button"
                          onclick={() => { taskLimitValue = 1; taskLimitUnit = 'MB/s'; applyTaskLimit(); }}
                          class="px-1.5 py-0.5 rounded bg-[#141b2b] hover:bg-[#1a2236] text-[10px] font-mono text-slate-300 hover:text-[#00e5ff] border border-[#222d45]"
                        >
                          1 MB/s
                        </button>
                        <button
                          type="button"
                          onclick={() => { taskLimitValue = 2; taskLimitUnit = 'MB/s'; applyTaskLimit(); }}
                          class="px-1.5 py-0.5 rounded bg-[#141b2b] hover:bg-[#1a2236] text-[10px] font-mono text-slate-300 hover:text-[#00e5ff] border border-[#222d45]"
                        >
                          2 MB/s
                        </button>
                        <button
                          type="button"
                          onclick={() => { taskLimitValue = 5; taskLimitUnit = 'MB/s'; applyTaskLimit(); }}
                          class="px-1.5 py-0.5 rounded bg-[#141b2b] hover:bg-[#1a2236] text-[10px] font-mono text-slate-300 hover:text-[#00e5ff] border border-[#222d45]"
                        >
                          5 MB/s
                        </button>
                      </div>
                    </div>
                  {/if}

                  <div class="pt-2 border-t border-[#1a2236] flex items-center justify-between text-[11px]">
                    <span class="text-slate-400">Batas Global Keseluruhan:</span>
                    <span class="font-mono {store.speedLimiterEnabled ? 'text-[#00e5ff] font-semibold' : 'text-slate-400'}">
                      {store.speedLimiterEnabled ? `Aktif (${store.globalSpeedLimitValue} ${store.globalSpeedLimitUnit})` : 'Tak Terbatas'}
                    </span>
                  </div>
                </div>
              </section>
            {:else if activeTab === 'options'}
              <section class="space-y-2 text-xs">
                <div class="bg-[#0f1420]/70 p-3 rounded-xl border border-[#1a2236] space-y-2">
                  <div class="flex items-start gap-1.5 text-slate-300 truncate">
                    <span class="text-slate-400 shrink-0 font-medium">Simpan Ke:</span>
                    <span class="font-mono text-[11px] text-[#00e5ff] truncate" title={task.file_path}>
                      {task.file_path}
                    </span>
                  </div>
                  <div class="pt-1.5 border-t border-[#1a2236] text-[11px] text-slate-400">
                    Unduhan akan disimpan otomatis ke folder di atas saat progres 100%.
                  </div>
                </div>
              </section>
            {/if}
          </div>

          <!-- Dual Progress Bars Section -->
          <div class="px-3.5 pb-2 space-y-2">
            <!-- Overall Download Progress -->
            <div class="space-y-1">
              <div class="flex justify-between items-center text-[11px] text-slate-400 px-0.5">
                <span class="font-medium text-slate-300">Total Progres</span>
                <span class="font-mono text-[#00e5ff] font-bold">{pct.toFixed(1)}%</span>
              </div>
              <div class="relative w-full h-4 bg-[#07090e] rounded border border-[#222d45] overflow-hidden flex items-center">
                <div
                  class="relative h-full bg-gradient-to-r from-emerald-600 via-emerald-500 to-green-400 bg-striped transition-[width] duration-150 ease-out will-change-[width] shadow-glow-emerald"
                  style="width: {pct}%"
                >
                  <div class="absolute inset-0 bg-gradient-to-r from-transparent via-white/20 to-transparent animate-shimmer pointer-events-none"></div>
                </div>
              </div>
            </div>

            <!-- IDM Segmented Multi-Thread Bar -->
            <div class="space-y-1 pt-0.5">
              <div class="flex justify-between items-center text-[10px] text-slate-400">
                <span>Multi-Thread Chunk Buffers</span>
                <span class="font-mono text-[#00e5ff]">{task.connections || 1} Jalur</span>
              </div>
              <div class="w-full h-3 bg-[#07090e] border border-[#222d45] rounded overflow-hidden flex items-center p-0.5 gap-0.5">
                {#if task.segments && task.segments.length > 0}
                  {#each task.segments as seg (seg.index)}
                    {@const segTotal = seg.end_byte - seg.start_byte + 1}
                    {@const segPct = segTotal > 0 ? (seg.downloaded_bytes / segTotal) * 100 : 0}
                    <div
                      class="h-full rounded-xs {seg.is_finished ? 'bg-[#10b981]' : isDownloading && segPct > 0 ? 'bg-gradient-to-r from-blue-700 to-[#00e5ff] shadow-[0_0_6px_rgba(0,229,255,0.4)]' : 'bg-[#141b2b]'}"
                      style="flex: 1;"
                      title="Jalur #{seg.index + 1}: {segPct.toFixed(0)}%"
                    ></div>
                  {/each}
                {:else}
                  <div
                    class="h-full bg-gradient-to-r from-blue-700 to-[#00e5ff] rounded-xs transition-[width] duration-150"
                    style="width: {pct}%"
                  ></div>
                {/if}
              </div>
            </div>
          </div>
        </div>

        <!-- In-Flight Transfer Bottom Control Toolbar -->
        <div class="p-3 border-t border-[#1f242e] bg-[#0b0f17] flex items-center justify-between gap-2">
          <div class="text-[11px] font-mono text-slate-400 truncate">
            {isDownloading ? formatSpeed(task.speed_bps) : 'Dijeda'}
          </div>

          <div class="flex items-center gap-2">
            {#if isDownloading}
              <button
                type="button"
                onclick={() => store.pauseTask(task.id)}
                class="px-3.5 py-1.5 text-xs font-semibold bg-[#141b2b] hover:bg-[#1a2236] text-amber-300 hover:text-amber-200 rounded-lg border border-[#222d45] transition cursor-pointer flex items-center gap-1"
              >
                <Pause class="w-3.5 h-3.5" />
                <span>Jeda</span>
              </button>
            {:else}
              <button
                type="button"
                onclick={() => store.resumeTask(task.id)}
                class="px-3.5 py-1.5 text-xs font-semibold bg-[#141b2b] hover:bg-[#1a2236] text-emerald-400 hover:text-emerald-300 rounded-lg border border-[#222d45] transition cursor-pointer flex items-center gap-1"
              >
                <Play class="w-3.5 h-3.5 fill-current" />
                <span>Lanjutkan</span>
              </button>
            {/if}

            <button
              type="button"
              onclick={() => { store.cancelTask(task.id, false); closeWindow(); }}
              class="px-3.5 py-1.5 text-xs font-semibold bg-red-950/40 hover:bg-red-900/60 text-red-200 rounded-lg border border-red-800/60 transition cursor-pointer"
            >
              Batal
            </button>
          </div>
        </div>
      </div>
    {/if}
  {/if}
</main>
