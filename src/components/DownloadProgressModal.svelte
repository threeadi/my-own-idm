<script lang="ts">
  import { store } from '$lib/idmStore.svelte';
  import { formatBytes, formatEta, formatSpeed, type DownloadTask, type SpeedLimitUnit, bpsToUnit, unitToBps } from '$lib/types';
  import {
    Download,
    X,
    Minus,
    Square,
    Play,
    Pause,
    CheckCircle2,
    Sliders,
    Settings,
    Gauge,
    Check,
    FolderOpen,
    Zap,
    ChevronDown,
    ChevronUp
  } from '@lucide/svelte';

  const task = $derived(store.progressModalTask);
  let activeTab = $state<'status' | 'limiter' | 'options'>('status');
  let showDetails = $state<boolean>(true);
  let taskLimiterEnabled = $state<boolean>(false);
  let taskLimitValue = $state<number>(1);
  let taskLimitUnit = $state<SpeedLimitUnit>('MB/s');
  let showCompleteDialog = $state<boolean>(true);
  let shutdownAfterComplete = $state<boolean>(false);

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
</script>

{#if store.isProgressModalOpen && task}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-3 sm:p-4 bg-black/40 backdrop-blur-[2px] select-none animate-in fade-in duration-150">
    <div class="relative w-full max-w-[620px] bg-[#0b0f17] border border-[#222d45] rounded-xl shadow-window overflow-hidden flex flex-col transition-all duration-300">
      <!-- Title Bar -->
      <header data-tauri-drag-region class="flex items-center justify-between px-3.5 py-2.5 bg-[#0f1420] border-b border-[#1a2236] select-none cursor-move">
        <div class="flex items-center gap-2 overflow-hidden pr-2">
          <img
            src="/favicon.png"
            alt="IDM Turbo"
            class="w-5 h-5 rounded-md shrink-0 object-contain shadow-[0_0_8px_rgba(0,229,255,0.3)]"
          />
          <h1 class="text-xs sm:text-[13px] font-medium text-slate-200 tracking-tight truncate flex items-center gap-1.5">
            <span class="font-bold text-[#00e5ff] font-mono">{pct.toFixed(0)}%</span>
            <span class="truncate">{task.filename}</span>
          </h1>
        </div>

        <div class="flex items-center gap-1 shrink-0">
          <button
            onclick={() => store.closeProgressModal()}
            class="w-6 h-6 flex items-center justify-center rounded text-slate-400 hover:text-slate-200 hover:bg-[#1a2236] transition-colors cursor-pointer"
            title="Minimalkan"
            type="button"
          >
            <Minus class="w-3.5 h-3.5" />
          </button>
          <button
            onclick={() => store.closeProgressModal()}
            class="w-6 h-6 flex items-center justify-center rounded text-slate-400 hover:text-white hover:bg-red-600/80 transition-colors cursor-pointer"
            title="Tutup"
            type="button"
          >
            <X class="w-3.5 h-3.5" />
          </button>
        </div>
      </header>

      <!-- Tab Navigation -->
      <nav class="flex items-center bg-[#0b0f17] border-b border-[#1a2236] px-2 pt-1.5 select-none text-xs">
        <button
          onclick={() => (activeTab = 'status')}
          class="flex items-center gap-1.5 px-3 py-1.5 font-medium transition-all cursor-pointer {activeTab === 'status' ? 'text-[#00e5ff] border-b-2 border-[#00e5ff] bg-[#0f1420]/80 rounded-t-md font-semibold' : 'text-slate-400 hover:text-slate-200 border-b-2 border-transparent'}"
        >
          <Download class="w-3.5 h-3.5" />
          <span>Status unduhan</span>
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
          <span>Opsi saat selesai</span>
        </button>
      </nav>

      <!-- Tab Content Area -->
      <div class="p-3.5 space-y-3 min-h-[160px]">
        {#if activeTab === 'status'}
          <!-- Status Tab Rows -->
          <section class="space-y-1.5 text-xs">
            <div class="bg-[#0f1420]/70 p-3 rounded-lg border border-[#1a2236] space-y-1.5">
              <div class="flex items-center justify-between">
                <span class="text-slate-400">Status:</span>
                <span class="font-medium {isCompleted ? 'text-emerald-400' : isDownloading ? 'text-[#00e5ff]' : 'text-amber-400'} capitalize">
                  {typeof task.status === 'string' ? task.status : 'Gagal'}
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
                <span class="text-slate-400">Kemampuan Melanjutkan:</span>
                <span class="inline-flex items-center gap-1 text-[#10b981] font-medium">
                  <CheckCircle2 class="w-3.5 h-3.5" />
                  <span>{task.supports_range ? 'Ya (Didukung Server)' : 'Tidak (Single Stream)'}</span>
                </span>
              </div>
            </div>
          </section>
        {:else if activeTab === 'limiter'}
          <section class="space-y-2.5 text-xs">
            <div class="bg-[#0f1420]/70 p-3 rounded-lg border border-[#1a2236] space-y-3">
              <div class="flex items-center justify-between">
                <span class="text-slate-300 font-medium">Tingkat transfer saat ini:</span>
                <span class="font-mono text-[#00e5ff] font-bold">{formatSpeed(task.speed_bps)}</span>
              </div>

              <!-- Per-Task Limiter Toggle -->
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

                  <!-- Quick Presets -->
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

              <!-- Global Limiter Status Notice -->
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
            <div class="bg-[#0f1420]/70 p-3 rounded-lg border border-[#1a2236] space-y-2">
              <div class="flex items-start gap-1.5 text-slate-300 truncate">
                <span class="text-slate-400 shrink-0 font-medium">Simpan Ke:</span>
                <span class="font-mono text-[11px] text-[#00e5ff] truncate" title={task.file_path}>
                  {task.file_path}
                </span>
              </div>

              <label class="flex items-center gap-2 cursor-pointer select-none pt-1">
                <input
                  type="checkbox"
                  bind:checked={showCompleteDialog}
                  class="w-4 h-4 rounded bg-[#0b0f17] border-[#222d45] text-[#00e5ff] focus:ring-0 cursor-pointer"
                />
                <span class="text-slate-200">Tampilkan dialog unduhan selesai</span>
              </label>

              <div class="pt-1.5 border-t border-[#1a2236] text-[11px] text-slate-400">
                <span>Tindakan otomatis pasca unduh:</span>
              </div>

              <div class="space-y-1 pl-1 text-[11px]">
                <label class="flex items-center gap-2 cursor-pointer select-none">
                  <input
                    type="checkbox"
                    bind:checked={shutdownAfterComplete}
                    class="w-3.5 h-3.5 rounded bg-[#0b0f17] border-[#222d45] text-[#00e5ff] focus:ring-0 cursor-pointer"
                  />
                  <span class="text-slate-300">Buka berkas secara otomatis setelah selesai</span>
                </label>
              </div>
            </div>
          </section>
        {/if}
      </div>

      <!-- Dual Progress Bars Section -->
      <div class="px-3.5 pb-3 space-y-2.5">
        <!-- Bar 1: Main Overall Download Progress -->
        <div class="space-y-1">
          <div class="flex justify-between items-center text-[11px] text-slate-400 px-0.5">
            <span class="font-medium text-slate-300">Total Progres</span>
            <span class="font-mono text-[#00e5ff] font-bold">{pct.toFixed(1)}%</span>
          </div>
          <div class="relative w-full h-5 bg-[#07090e] rounded border border-[#222d45] overflow-hidden flex items-center">
            <div
              class="relative h-full bg-gradient-to-r from-emerald-600 via-emerald-500 to-green-400 bg-striped transition-all duration-200 shadow-glow-emerald"
              style="width: {pct}%"
            >
              <div class="absolute inset-0 bg-gradient-to-r from-transparent via-white/20 to-transparent animate-shimmer"></div>
            </div>
          </div>
        </div>

        <!-- Buttons Row: Toggle Details & Actions -->
        <div class="flex items-center justify-between gap-2 pt-0.5">
          <button
            onclick={() => (showDetails = !showDetails)}
            class="inline-flex items-center gap-1 px-2.5 py-1 text-xs font-medium bg-[#141b2b] hover:bg-[#1a2236] text-slate-300 hover:text-white rounded border border-[#222d45] transition cursor-pointer"
          >
            {#if showDetails}
              <ChevronUp class="w-3.5 h-3.5 text-[#00e5ff]" />
              <span>Sembunyikan detail</span>
            {:else}
              <ChevronDown class="w-3.5 h-3.5 text-[#00e5ff]" />
              <span>Tampilkan detail</span>
            {/if}
          </button>

          <div class="flex items-center gap-2">
            {#if isDownloading}
              <button
                onclick={() => store.pauseTask(task.id)}
                class="px-3 py-1 text-xs font-semibold bg-[#141b2b] hover:bg-[#1a2236] text-amber-300 hover:text-amber-200 rounded border border-[#222d45] transition cursor-pointer"
                type="button"
              >
                Jeda
              </button>
            {:else if !isCompleted}
              <button
                onclick={() => store.resumeTask(task.id)}
                class="px-3 py-1 text-xs font-semibold bg-[#141b2b] hover:bg-[#1a2236] text-emerald-400 hover:text-emerald-300 rounded border border-[#222d45] transition cursor-pointer"
                type="button"
              >
                Mulai
              </button>
            {/if}

            <button
              onclick={() => { store.cancelTask(task.id, false); store.closeProgressModal(); }}
              class="px-3 py-1 text-xs font-semibold bg-red-950/40 hover:bg-red-900/60 text-red-200 rounded border border-red-800/60 transition cursor-pointer"
              type="button"
            >
              Batal
            </button>
          </div>
        </div>

        <!-- Bar 2: IDM Multi-Connection Segmented Buffer Bar -->
        <div class="space-y-1 pt-1">
          <div class="flex justify-between items-center text-[11px] text-slate-400">
            <span>Progres posisi mulai dan unduh berdasarkan koneksi</span>
            <span class="text-[10px] font-mono text-[#00e5ff]">{task.connections || 1} Threads</span>
          </div>
          <div class="w-full h-4 bg-[#07090e] border border-[#222d45] rounded overflow-hidden flex items-center p-0.5 gap-0.5">
            {#if task.segments && task.segments.length > 0}
              {#each task.segments as seg}
                {@const segTotal = seg.end_byte - seg.start_byte + 1}
                {@const segPct = segTotal > 0 ? Math.min(100, Math.max(0, (seg.downloaded_bytes / segTotal) * 100)) : 0}
                <div
                  class="h-full bg-[#141b2b] rounded-xs overflow-hidden flex-1 relative border border-[#222d45]/50 flex items-center"
                  title="Bagian #{seg.index + 1}: {segPct.toFixed(0)}% ({formatBytes(seg.downloaded_bytes)} / {formatBytes(segTotal)})"
                >
                  <div
                    class="h-full transition-[width] duration-150 ease-out {seg.is_finished ? 'bg-[#10b981]' : isDownloading && segPct > 0 ? 'bg-gradient-to-r from-blue-700 to-[#00e5ff] shadow-[0_0_6px_rgba(0,229,255,0.4)]' : 'bg-transparent'}"
                    style="width: {segPct}%;"
                  ></div>
                </div>
              {/each}
            {:else}
              <div class="h-full bg-[#141b2b] rounded-xs overflow-hidden w-full relative">
                <div
                  class="h-full bg-gradient-to-r from-blue-700 to-[#00e5ff] rounded-xs transition-all"
                  style="width: {pct}%"
                ></div>
              </div>
            {/if}
          </div>
        </div>
      </div>

      <!-- Connection Threads Live Telemetry Table -->
      {#if showDetails && task.segments && task.segments.length > 0}
        <div class="px-3.5 pb-3 transition-all duration-300">
          <div class="border border-[#222d45] rounded-lg overflow-hidden bg-[#07090e]/80 max-h-36 overflow-y-auto">
            <table class="w-full text-left border-collapse text-xs">
              <thead class="bg-[#0f1420] text-slate-400 border-b border-[#1a2236] text-[10px] uppercase select-none">
                <tr>
                  <th class="py-1 px-3 w-12 text-center">N.</th>
                  <th class="py-1 px-3 w-32">Diunduh</th>
                  <th class="py-1 px-3">Info</th>
                </tr>
              </thead>
              <tbody class="font-mono divide-y divide-[#141b2b] text-slate-300 text-xs">
                {#each task.segments as seg}
                  {@const segTotal = seg.end_byte - seg.start_byte + 1}
                  <tr class="hover:bg-[#141b2b]/50 transition-colors">
                    <td class="py-1 px-3 text-center text-slate-500 font-sans">{seg.index + 1}</td>
                    <td class="py-1 px-3 text-[#00e5ff] font-medium">{formatBytes(seg.downloaded_bytes)}</td>
                    <td class="py-1 px-3">
                      {#if isCompleted || seg.is_finished}
                        <span class="inline-flex items-center gap-1 text-[#10b981] font-sans text-[11px]">
                          <Check class="w-3 h-3" />
                          <span>Selesai.</span>
                        </span>
                      {:else if isDownloading && seg.downloaded_bytes > 0}
                        <span class="inline-flex items-center gap-1 text-emerald-400 font-sans text-[11px]">
                          <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-ping"></span>
                          <span>Mengunduh... (Aktif)</span>
                        </span>
                      {:else}
                        <span class="inline-flex items-center gap-1 text-slate-500 font-sans text-[11px]">
                          <span class="w-1.5 h-1.5 rounded-full bg-slate-500"></span>
                          <span>Siap.</span>
                        </span>
                      {/if}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>
      {/if}
    </div>
  </div>
{/if}
