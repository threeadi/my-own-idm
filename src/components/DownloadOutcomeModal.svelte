<script lang="ts">
  import { store } from "$lib/idmStore.svelte";
  import { formatBytes, type DownloadTask } from "$lib/types";
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
  } from "@lucide/svelte";

  const task = $derived(store.outcomeTask);
  const isOpen = $derived(store.isOutcomeModalOpen && !!task);
  const isCompleted = $derived(store.outcomeType === "completed");

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
</script>

{#if isOpen && task}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-3 sm:p-4 bg-black/40 backdrop-blur-[2px] select-none animate-in fade-in duration-150"
  >
    <div
      class="relative w-full max-w-[540px] bg-[#0f141c] border border-[#252a33] rounded-2xl shadow-2xl overflow-hidden flex flex-col transition-all duration-200"
      id="download-outcome-modal"
    >
      <!-- Top Colored Accent Stripe -->
      <div
        class="absolute inset-x-0 top-0 h-[2px] {isCompleted
          ? 'bg-gradient-to-r from-transparent via-[#10b981] to-transparent'
          : 'bg-gradient-to-r from-transparent via-[#ff5252] to-transparent'}"
      ></div>

      <!-- Title Bar -->
      <header
        data-tauri-drag-region
        class="px-4 py-3 bg-[#171c24] flex items-center justify-between border-b border-[#252a33] cursor-move"
      >
        <div class="flex items-center gap-2.5">
          {#if isCompleted}
            <div
              class="w-6 h-6 rounded-lg bg-[#10b981]/20 flex items-center justify-center text-[#10b981] border border-[#10b981]/30"
            >
              <CheckCircle2 class="w-4 h-4" />
            </div>
            <span class="text-sm font-semibold text-[#dee2ee] tracking-tight"
              >Unduhan Selesai</span
            >
            <span
              class="px-2 py-0.5 rounded-full bg-[#10b981]/15 text-[#10b981] border border-[#10b981]/30 text-[10px] font-bold tracking-wider uppercase"
            >
              100% Selesai
            </span>
          {:else}
            <div
              class="w-6 h-6 rounded-lg bg-[#ff5252]/20 flex items-center justify-center text-[#ff5252] border border-[#ff5252]/30"
            >
              <AlertTriangle class="w-4 h-4" />
            </div>
            <span class="text-sm font-semibold text-[#dee2ee] tracking-tight"
              >Unduhan Gagal</span
            >
            <span
              class="px-2 py-0.5 rounded-full bg-[#ff5252]/15 text-[#ff5252] border border-[#ff5252]/30 text-[10px] font-bold tracking-wider uppercase"
            >
              Gagal
            </span>
          {/if}
        </div>

        <button
          onclick={() => store.closeOutcomeModal()}
          class="w-7 h-7 flex items-center justify-center rounded-lg text-slate-400 hover:text-slate-100 hover:bg-[#252a33] transition-colors cursor-pointer"
          title="Tutup"
        >
          <X class="w-4 h-4" />
        </button>
      </header>

      <!-- Content Body -->
      <div class="p-5 space-y-4">
        <!-- File Info Card -->
        <div
          class="flex items-start gap-4 p-4 rounded-xl bg-[#171c24] border border-[#252a33]"
        >
          <div
            class="w-12 h-12 rounded-xl flex items-center justify-center shrink-0 {isCompleted
              ? 'bg-[#10b981]/10 text-[#10b981] border border-[#10b981]/20'
              : 'bg-[#ff5252]/10 text-[#ff5252] border border-[#ff5252]/20'}"
          >
            <FileIcon class="w-6 h-6" />
          </div>

          <div class="flex-1 min-w-0 space-y-1.5">
            <h2
              class="text-sm font-bold text-white truncate"
              title={task.filename}
            >
              {task.filename}
            </h2>

            <div
              class="flex flex-wrap items-center gap-2 text-xs text-slate-400 font-mono"
            >
              <span
                class="px-2 py-0.5 rounded bg-[#090e16] text-[#00e5ff] font-bold"
              >
                {formatBytes(task.downloaded_bytes || task.total_bytes || 0)}
              </span>
              <span>•</span>
              <span
                class="uppercase text-slate-300 font-sans font-semibold text-[11px]"
              >
                {task.category}
              </span>
              {#if isCompleted}
                <span>•</span>
                <span
                  class="text-[#10b981] flex items-center gap-1 font-sans text-[11px]"
                >
                  <ShieldCheck class="w-3.5 h-3.5" /> Berkas Utuh
                </span>
              {/if}
            </div>

            <div
              class="text-[11px] text-slate-400 truncate flex items-center gap-1 pt-0.5 font-sans"
              title={task.file_path}
            >
              <Folder class="w-3.5 h-3.5 text-slate-500 shrink-0" />
              <span class="truncate">{task.file_path}</span>
            </div>
          </div>
        </div>

        <!-- Failure Reason Box (if failed) -->
        {#if !isCompleted}
          <div
            class="p-3.5 rounded-xl bg-[#ff5252]/10 border border-[#ff5252]/25 text-xs text-[#ffb4ab] space-y-1"
          >
            <div class="font-bold flex items-center gap-1.5 text-rose-300">
              <AlertTriangle class="w-3.5 h-3.5" />
              Alasan Kegagalan:
            </div>
            <div
              class="font-mono text-[11px] leading-relaxed break-words text-rose-200/90 pl-5"
            >
              {store.outcomeErrorMessage ||
                task.error_message ||
                "Koneksi ke server terputus atau server menolak permintaan."}
            </div>
          </div>
        {/if}
      </div>

      <!-- Action Footer -->
      <footer
        class="px-5 py-3.5 bg-[#171c24] border-t border-[#252a33] flex items-center justify-between gap-2.5"
      >
        {#if isCompleted}
          <button
            onclick={handleOpenFolder}
            class="px-3.5 py-2 rounded-xl text-xs font-semibold text-slate-300 bg-[#252a33] hover:bg-[#30353e] hover:text-white transition-all flex items-center gap-1.5 cursor-pointer"
          >
            <FolderOpen class="w-3.5 h-3.5" />
            Buka Folder
          </button>

          <div class="flex items-center gap-2">
            <button
              onclick={() => store.closeOutcomeModal()}
              class="px-4 py-2 rounded-xl text-xs font-semibold text-slate-400 hover:text-slate-200 hover:bg-[#252a33] transition-all cursor-pointer"
            >
              Tutup
            </button>
            <button
              onclick={handleOpenFile}
              class="px-5 py-2 rounded-xl text-xs font-bold text-slate-950 bg-gradient-to-r from-[#00e5ff] to-[#4d8eff] hover:opacity-95 shadow-glow-cyan transition-all flex items-center gap-1.5 cursor-pointer"
            >
              <Play class="w-3.5 h-3.5 fill-current" />
              Buka Berkas
            </button>
          </div>
        {:else}
          <button
            onclick={() => store.closeOutcomeModal()}
            class="px-4 py-2 rounded-xl text-xs font-semibold text-slate-400 hover:text-slate-200 hover:bg-[#252a33] transition-all cursor-pointer"
          >
            Tutup
          </button>
          <button
            onclick={handleRetry}
            class="px-5 py-2 rounded-xl text-xs font-bold text-white bg-[#2563eb] hover:bg-[#1d4ed8] shadow-md transition-all flex items-center gap-1.5 cursor-pointer"
          >
            <RotateCcw class="w-3.5 h-3.5" />
            Coba Lagi
          </button>
        {/if}
      </footer>
    </div>
  </div>
{/if}
