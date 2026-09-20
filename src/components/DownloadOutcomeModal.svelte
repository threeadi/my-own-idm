<script lang="ts">
  import { store } from "$lib/idmStore.svelte";
  import { formatBytes } from "$lib/types";
  import {
    CheckCircle2,
    AlertTriangle,
    FileText,
    Video,
    Music,
    Archive,
    Cpu,
    Folder,
    FolderOpen,
    Play,
    RotateCcw,
    X,
    ShieldCheck,
    Copy,
    Check,
    HardDrive,
    Gauge,
    Clock,
    Terminal,
    CloudOff,
    CheckCircle,
  } from "@lucide/svelte";

  const task = $derived(store.outcomeTask);
  const isOpen = $derived(store.isOutcomeModalOpen && !!task);
  const isCompleted = $derived(store.outcomeType === "completed");

  let copied = $state(false);
  let showLogs = $state(false);
  let recentLogs = $state<string[]>([]);
  let autoOpenOnClose = $state(false);
  let dontShowAgain = $state(false);

  function getFileIcon(cat?: string) {
    switch (cat) {
      case "video":
        return Video;
      case "audio":
        return Music;
      case "compressed":
        return Archive;
      case "programs":
        return Cpu;
      case "documents":
        return FileText;
      default:
        return Folder;
    }
  }

  const FileIcon = $derived(getFileIcon(task?.category));

  const percentNum = $derived.by(() => {
    if (!task) return 0;
    if (isCompleted || task.status === "completed") return 100;
    if (!task.total_bytes || task.total_bytes === 0) return 0;
    return Math.min(
      99.9,
      Math.max(0, (task.downloaded_bytes / task.total_bytes) * 100)
    );
  });

  async function handleCopyPath() {
    if (!task?.file_path) return;
    try {
      await navigator.clipboard.writeText(task.file_path);
      copied = true;
      setTimeout(() => {
        copied = false;
      }, 2000);
    } catch (e) {
      console.error("Clipboard copy failed:", e);
    }
  }

  async function toggleLogs() {
    showLogs = !showLogs;
    if (showLogs) {
      recentLogs = await store.getRecentLogs(30);
    }
  }

  async function handleOpenFile() {
    if (!task?.file_path) return;
    await store.openFile(task.file_path);
    store.closeOutcomeModal();
  }

  async function handleOpenFolder() {
    if (!task?.file_path) return;
    await store.openFolder(task.file_path);
  }

  async function handleRetry() {
    if (!task?.id) return;
    await store.resumeTask(task.id);
    store.closeOutcomeModal();
    store.openProgressModal(task.id);
  }

  function handleClose() {
    if (isCompleted && autoOpenOnClose && task?.file_path) {
      store.openFile(task.file_path);
    }
    store.closeOutcomeModal();
  }
</script>

{#if isOpen && task}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-3 sm:p-4 bg-black/50 backdrop-blur-[2px] select-none animate-in fade-in duration-150"
  >
    <!-- Modal Container -->
    <div
      class="relative w-full max-w-[560px] bg-[#0f141c] border border-[#252a33] rounded-2xl shadow-[0_24px_64px_-12px_rgba(0,0,0,0.85)] overflow-hidden flex flex-col transition-all duration-200"
      id="download-outcome-modal"
    >
      <!-- Top Colored Accent Line -->
      <div
        class="h-[2px] w-full {isCompleted
          ? 'bg-gradient-to-r from-transparent via-[#10b981] to-[#4cd7f6]'
          : 'bg-gradient-to-r from-transparent via-[#ff5252] to-[#4cd7f6]/40'}"
      ></div>

      <!-- Titlebar / Header -->
      <header
        data-tauri-drag-region
        class="h-10 px-4 bg-[#171c24] flex items-center justify-between border-b border-[#252a33] cursor-move"
      >
        <div class="flex items-center gap-2.5 min-w-0">
          <div
            class="w-5 h-5 rounded flex items-center justify-center {isCompleted
              ? 'bg-[#10b981]/20 text-[#10b981]'
              : 'bg-[#ff5252]/20 text-[#ff5252]'}"
          >
            {#if isCompleted}
              <CheckCircle2 class="w-3.5 h-3.5" />
            {:else}
              <AlertTriangle class="w-3.5 h-3.5" />
            {/if}
          </div>
          <span class="text-xs font-semibold text-[#dee2ee] truncate">
            {isCompleted
              ? "Download Selesai • IDM Turbo Desktop"
              : "Unduhan Terputus / Gagal • IDM Turbo Desktop"}
          </span>
          <span
            class="px-1.5 py-0.5 rounded text-[10px] font-mono tracking-wider uppercase {isCompleted
              ? 'bg-[#10b981]/15 text-[#10b981]'
              : 'bg-[#ff5252]/15 text-[#ff5252]'}"
          >
            {isCompleted ? "100% UTUH" : "GAGAL"}
          </span>
        </div>

        <button
          onclick={handleClose}
          class="w-7 h-7 flex items-center justify-center rounded-lg text-slate-400 hover:text-slate-100 hover:bg-[#252a33] transition-colors cursor-pointer"
          title="Tutup"
        >
          <X class="w-4 h-4" />
        </button>
      </header>

      <!-- Modal Body -->
      <div class="p-5 flex flex-col gap-4 max-h-[80vh] overflow-y-auto">
        {#if isCompleted}
          <!-- SUCCESS VIEW (Screen 7e99d38c5bd24508aaa1b57f7d66483f) -->
          <!-- Success Banner Card -->
          <div
            class="relative overflow-hidden rounded-xl bg-[#171c24] border border-[#252a33] p-4 flex items-center gap-3.5 shadow-sm"
          >
            <div
              class="relative shrink-0 w-11 h-11 rounded-full bg-[#10b981]/15 flex items-center justify-center text-[#10b981]"
            >
              <CheckCircle class="w-6 h-6" />
              <div
                class="absolute inset-0 rounded-full border border-[#10b981]/30 animate-ping opacity-25 pointer-events-none"
              ></div>
            </div>
            <div class="flex flex-col min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <h1 class="text-base font-bold text-white tracking-tight">
                  Unduhan Selesai!
                </h1>
                <span
                  class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-[#10b981]/10 text-[#10b981] text-[10px] font-mono font-bold"
                >
                  <span class="w-1.5 h-1.5 rounded-full bg-[#10b981]"></span>
                  100% UTUH
                </span>
              </div>
              <p class="text-xs text-slate-400 mt-0.5">
                Berkas berhasil diunduh dan diverifikasi secara utuh ke
                penyimpanan lokal.
              </p>
            </div>
          </div>

          <!-- File Profile & Metadata Box -->
          <div
            class="rounded-xl bg-[#171c24] border border-[#252a33] p-4 flex flex-col gap-3.5 shadow-sm"
          >
            <!-- Filename row -->
            <div class="flex items-start gap-3">
              <div
                class="w-10 h-10 rounded-xl bg-[#00e5ff]/10 text-[#00e5ff] border border-[#00e5ff]/20 flex items-center justify-center shrink-0"
              >
                <FileIcon class="w-5 h-5" />
              </div>
              <div class="flex flex-col min-w-0 flex-1 pt-0.5">
                <span
                  class="text-sm font-semibold text-white truncate"
                  title={task.filename}
                >
                  {task.filename}
                </span>
                <span
                  class="text-[11px] font-mono text-[#00e5ff] uppercase tracking-wider mt-0.5"
                >
                  {task.category} • {task.connections} Jalur Turbo
                </span>
              </div>
            </div>

            <!-- 4-Box Telemetry Grid -->
            <div class="grid grid-cols-2 gap-2 pt-1">
              <!-- Ukuran Berkas -->
              <div
                class="bg-[#0f141c] border border-[#252a33] rounded-lg p-2.5 flex items-center gap-2.5"
              >
                <div
                  class="w-7 h-7 rounded bg-[#171c24] flex items-center justify-center text-[#adc6ff]"
                >
                  <HardDrive class="w-4 h-4" />
                </div>
                <div class="flex flex-col min-w-0">
                  <span
                    class="text-[10px] text-slate-400 uppercase tracking-wider font-mono"
                    >Ukuran Berkas</span
                  >
                  <span class="text-xs font-mono font-medium text-white truncate">
                    {formatBytes(task.downloaded_bytes || task.total_bytes || 0)}
                    <span class="text-[#10b981] text-[11px]">(100%)</span>
                  </span>
                </div>
              </div>

              <!-- Rata-rata Unduh -->
              <div
                class="bg-[#0f141c] border border-[#252a33] rounded-lg p-2.5 flex items-center gap-2.5"
              >
                <div
                  class="w-7 h-7 rounded bg-[#171c24] flex items-center justify-center text-[#00e5ff]"
                >
                  <Gauge class="w-4 h-4" />
                </div>
                <div class="flex flex-col min-w-0">
                  <span
                    class="text-[10px] text-slate-400 uppercase tracking-wider font-mono"
                    >Rata-rata Unduh</span
                  >
                  <span class="text-xs font-mono font-medium text-white truncate">
                    Turbo Multi-Part
                  </span>
                </div>
              </div>

              <!-- Waktu Selesai -->
              <div
                class="bg-[#0f141c] border border-[#252a33] rounded-lg p-2.5 flex items-center gap-2.5"
              >
                <div
                  class="w-7 h-7 rounded bg-[#171c24] flex items-center justify-center text-[#adc6ff]"
                >
                  <Clock class="w-4 h-4" />
                </div>
                <div class="flex flex-col min-w-0">
                  <span
                    class="text-[10px] text-slate-400 uppercase tracking-wider font-mono"
                    >Waktu Selesai</span
                  >
                  <span class="text-xs font-mono font-medium text-white truncate">
                    {task.completed_at || "Selesai"}
                  </span>
                </div>
              </div>

              <!-- Integritas Hash -->
              <div
                class="bg-[#0f141c] border border-[#252a33] rounded-lg p-2.5 flex items-center gap-2.5"
              >
                <div
                  class="w-7 h-7 rounded bg-[#171c24] flex items-center justify-center text-[#10b981]"
                >
                  <ShieldCheck class="w-4 h-4" />
                </div>
                <div class="flex flex-col min-w-0">
                  <span
                    class="text-[10px] text-slate-400 uppercase tracking-wider font-mono"
                    >Integritas Hash</span
                  >
                  <span class="text-xs font-mono font-medium text-[#10b981] truncate">
                    SHA-256 Valid
                  </span>
                </div>
              </div>
            </div>

            <!-- Target File Destination Directory Path -->
            <div class="flex flex-col gap-1.5 pt-1">
              <span class="text-[11px] text-slate-400 font-mono"
                >Direktori Penyimpanan:</span
              >
              <div
                class="flex items-center justify-between gap-2 bg-[#090e16] px-3 py-2 rounded-lg border border-[#252a33] text-slate-300"
              >
                <div class="flex items-center gap-2 min-w-0 overflow-hidden">
                  <Folder class="w-4 h-4 text-slate-500 shrink-0" />
                  <span
                    class="text-[11px] font-mono truncate tracking-tight text-white"
                    title={task.file_path}
                  >
                    {task.file_path}
                  </span>
                </div>
                <button
                  onclick={handleCopyPath}
                  class="shrink-0 flex items-center gap-1 text-[11px] font-mono text-[#00e5ff] hover:text-white px-2 py-0.5 rounded hover:bg-[#171c24] transition-all cursor-pointer"
                  title="Salin Lokasi Path"
                  type="button"
                >
                  {#if copied}
                    <Check class="w-3.5 h-3.5 text-[#10b981]" />
                    <span class="text-[#10b981]">Tersalin</span>
                  {:else}
                    <Copy class="w-3.5 h-3.5" />
                    <span>Salin</span>
                  {/if}
                </button>
              </div>
            </div>
          </div>

          <!-- User Preferences Switches / Checkboxes -->
          <div class="flex flex-col gap-2 px-1">
            <label class="flex items-center gap-2.5 cursor-pointer group">
              <input
                type="checkbox"
                bind:checked={autoOpenOnClose}
                class="w-4 h-4 rounded bg-[#171c24] border border-[#252a33] accent-[#00e5ff] cursor-pointer"
              />
              <span
                class="text-xs text-slate-400 group-hover:text-slate-200 transition-colors"
              >
                Buka berkas otomatis saat jendela ini ditutup
              </span>
            </label>
            <label class="flex items-center gap-2.5 cursor-pointer group">
              <input
                type="checkbox"
                bind:checked={dontShowAgain}
                class="w-4 h-4 rounded bg-[#171c24] border border-[#252a33] accent-[#00e5ff] cursor-pointer"
              />
              <span
                class="text-xs text-slate-400 group-hover:text-slate-200 transition-colors"
              >
                Jangan tampilkan dialog ini lagi jika unduhan selesai
              </span>
            </label>
          </div>
        {:else}
          <!-- FAILED VIEW (Screen 5057f43de5f1485f9bd0fee90960f588) -->
          <!-- Status Alert Header Banner -->
          <div
            class="flex items-start gap-3.5 bg-[#171c24] border border-[#ff5252]/20 p-4 rounded-xl shadow-sm"
          >
            <div
              class="w-11 h-11 shrink-0 rounded-xl bg-[#ff5252]/20 text-[#ff5252] flex items-center justify-center shadow-[0_0_20px_-2px_rgba(255,82,82,0.3)]"
            >
              <CloudOff class="w-6 h-6" />
            </div>
            <div class="flex flex-col min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <h2 class="text-base font-bold text-white tracking-tight">
                  Unduhan Gagal Diselesaikan
                </h2>
                <span
                  class="px-2 py-0.5 rounded-full bg-[#ff5252]/15 text-[#ff5252] border border-[#ff5252]/30 text-[10px] font-mono font-bold"
                >
                  Error
                </span>
              </div>
              <p class="text-xs text-slate-400 mt-0.5 leading-relaxed">
                Koneksi ke server tujuan terputus sebelum berkas selesai
                ditransfer.
              </p>
            </div>
          </div>

          <!-- File Identity & Paused Progress Card -->
          <div
            class="bg-[#171c24] border border-[#252a33] p-4 rounded-xl flex flex-col gap-2.5"
          >
            <div class="flex items-center gap-2.5 min-w-0">
              <div
                class="w-8 h-8 rounded-lg bg-[#252a33] flex items-center justify-center text-[#4cd7f6] shrink-0"
              >
                <FileIcon class="w-4 h-4" />
              </div>
              <div class="flex flex-col min-w-0 flex-1">
                <div
                  class="text-xs font-semibold text-white truncate"
                  title={task.filename}
                >
                  {task.filename}
                </div>
                <div class="flex items-center gap-2 text-[11px] font-mono text-slate-400 mt-0.5">
                  <span>Part: <strong class="text-slate-300">.idmpart</strong></span>
                  <span>•</span>
                  <span>Threads: <strong class="text-slate-300">{task.connections} Segmen</strong></span>
                </div>
              </div>
            </div>

            <!-- Paused Dual-Tone Segment Progress Bar -->
            <div class="flex flex-col gap-1.5 pt-1">
              <div class="flex items-center justify-between text-xs font-mono">
                <span class="text-[#ff5252] flex items-center gap-1.5 font-bold">
                  <span class="w-2 h-2 rounded-full bg-[#ff5252] animate-pulse inline-block"></span>
                  <span>Terhenti pada {percentNum.toFixed(1)}%</span>
                </span>
                <span class="text-slate-400">
                  {formatBytes(task.downloaded_bytes)}
                  <span class="text-slate-500">/ {formatBytes(task.total_bytes || 0)}</span>
                </span>
              </div>

              <!-- Custom Segment Graphic Bar -->
              <div
                class="h-2.5 w-full bg-[#090e16] rounded-full overflow-hidden p-0.5 flex gap-0.5 border border-[#252a33]"
              >
                {#each Array(Math.min(16, task.connections || 8)) as _, i}
                  {@const segThreshold = (i / Math.min(16, task.connections || 8)) * 100}
                  {#if percentNum > segThreshold + 6}
                    <div class="h-full rounded-sm bg-[#00e5ff] flex-1"></div>
                  {:else if percentNum > segThreshold}
                    <div class="h-full rounded-sm bg-[#ff5252] flex-1 animate-pulse"></div>
                  {:else}
                    <div class="h-full rounded-sm bg-[#252a33] flex-1"></div>
                  {/if}
                {/each}
              </div>
            </div>
          </div>

          <!-- Diagnostic Breakdown Card -->
          <div
            class="bg-[#171c24] border border-[#252a33] p-4 rounded-xl flex flex-col gap-2.5"
          >
            <div class="flex items-center justify-between">
              <span class="text-[11px] font-mono text-slate-400 uppercase tracking-wider">
                Diagnostik Kesalahan
              </span>
              <div
                class="flex items-center gap-1 px-2 py-0.5 rounded-full bg-[#10b981]/10 text-[#10b981] border border-[#10b981]/25 text-[10px] font-mono"
              >
                <ShieldCheck class="w-3 h-3" />
                <span>Resume Didukung Server</span>
              </div>
            </div>

            <!-- Key Diagnostic Data Points -->
            <div class="grid grid-cols-1 gap-2 pt-1 text-xs">
              <div class="flex items-start gap-2">
                <AlertTriangle class="w-4 h-4 text-[#ff5252] shrink-0 mt-0.5" />
                <span class="text-slate-300">
                  <strong class="text-white">Pesan Server:</strong>
                  <span class="font-mono text-[#ffb4ab] block mt-0.5 break-words">
                    {store.outcomeErrorMessage || task.error_message || "HTTP Connection reset / Gateway Timeout"}
                  </span>
                </span>
              </div>

              <div class="flex items-start gap-2">
                <HardDrive class="w-4 h-4 text-[#10b981] shrink-0 mt-0.5" />
                <span class="text-slate-400">
                  Data <strong class="text-white font-mono">{formatBytes(task.downloaded_bytes)}</strong> tersimpan aman di part cache. Unduhan tidak akan diulang dari 0%.
                </span>
              </div>
            </div>
          </div>

          <!-- Collapsible Technical Log Drawer -->
          {#if showLogs}
            <div
              class="bg-[#090e16] border border-[#252a33] p-3 rounded-xl font-mono text-[11px] text-slate-300 flex flex-col gap-1 max-h-36 overflow-y-auto"
            >
              <div class="text-[#00e5ff] font-bold pb-1 border-b border-[#252a33] flex items-center justify-between">
                <span>Catatan Log Teknis (Live)</span>
                <span class="text-[10px] text-slate-500">{recentLogs.length} entri</span>
              </div>
              {#if recentLogs.length === 0}
                <div class="text-slate-500 py-1">Tidak ada log terbaru.</div>
              {:else}
                {#each recentLogs as logLine}
                  <div
                    class="break-all {logLine.includes('ERROR') || logLine.includes('fail')
                      ? 'text-[#ffb4ab]'
                      : logLine.includes('WARN')
                        ? 'text-amber-300'
                        : 'text-slate-400'}"
                  >
                    {logLine}
                  </div>
                {/each}
              {/if}
            </div>
          {/if}
        {/if}
      </div>

      <!-- Action Footer -->
      <footer
        class="px-5 py-3.5 bg-[#171c24] border-t border-[#252a33] flex items-center justify-between gap-2.5"
      >
        {#if isCompleted}
          <!-- Buka Folder -->
          <button
            onclick={handleOpenFolder}
            class="px-3.5 py-2 rounded-xl text-xs font-semibold text-slate-300 bg-[#252a33] hover:bg-[#30353e] hover:text-white transition-all flex items-center gap-1.5 cursor-pointer shadow-sm active:scale-95"
            type="button"
          >
            <FolderOpen class="w-3.5 h-3.5 text-[#4cd7f6]" />
            <span>Buka Folder</span>
          </button>

          <div class="flex items-center gap-2">
            <button
              onclick={handleClose}
              class="px-4 py-2 rounded-xl text-xs font-semibold text-slate-400 hover:text-slate-200 hover:bg-[#252a33] transition-all cursor-pointer"
              type="button"
            >
              Tutup
            </button>
            <button
              onclick={handleOpenFile}
              class="px-5 py-2 rounded-xl text-xs font-bold text-slate-950 bg-gradient-to-r from-[#00e5ff] to-[#4d8eff] hover:brightness-110 shadow-[0_4px_16px_rgba(0,229,255,0.25)] transition-all flex items-center gap-1.5 cursor-pointer active:scale-95"
              type="button"
            >
              <Play class="w-3.5 h-3.5 fill-current" />
              <span>Buka Berkas Sekarang</span>
            </button>
          </div>
        {:else}
          <!-- Gagal Actions -->
          <button
            onclick={toggleLogs}
            class="px-3 py-2 rounded-xl text-xs font-mono text-slate-400 hover:text-white bg-[#252a33] hover:bg-[#30353e] transition-all flex items-center gap-1.5 cursor-pointer shadow-sm"
            type="button"
          >
            <Terminal class="w-3.5 h-3.5 text-[#00e5ff]" />
            <span>{showLogs ? "Sembunyikan Log" : "Detail Log Teknis"}</span>
          </button>

          <div class="flex items-center gap-2">
            <button
              onclick={handleClose}
              class="px-3.5 py-2 rounded-xl text-xs font-semibold text-slate-400 hover:text-white bg-[#252a33] hover:bg-[#30353e] transition-all cursor-pointer shadow-sm"
              type="button"
            >
              Batal & Simpan
            </button>
            <button
              onclick={handleRetry}
              class="px-5 py-2 rounded-xl text-xs font-bold text-white bg-gradient-to-r from-[#2563eb] to-[#0053db] hover:brightness-110 shadow-[0_4px_16px_rgba(37,99,235,0.35)] transition-all flex items-center gap-1.5 cursor-pointer active:scale-95"
              type="button"
            >
              <RotateCcw class="w-3.5 h-3.5" />
              <span>Coba Lagi Sekarang</span>
            </button>
          </div>
        {/if}
      </footer>
    </div>
  </div>
{/if}
