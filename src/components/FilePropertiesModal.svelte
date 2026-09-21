<script lang="ts">
  import { store, isTauri } from "$lib/idmStore.svelte";
  import { formatBytes, type SpeedLimitUnit, bpsToUnit, unitToBps } from "$lib/types";
  import { open } from "@tauri-apps/plugin-dialog";
  import {
    CheckCircle2,
    Video,
    Music,
    Archive,
    Cpu,
    FileText,
    Folder,
    FolderOpen,
    Play,
    X,
    ShieldCheck,
    Copy,
    Check,
    Link,
    HardDrive,
    MoveRight,
    FileCode,
    ExternalLink,
    Clock,
    Gauge,
  } from "@lucide/svelte";

  const task = $derived(store.propertiesTask);
  const isOpen = $derived(store.isPropertiesModalOpen && !!task);

  let copiedUrl = $state(false);
  let isMoving = $state(false);
  let moveSuccess = $state(false);
  let moveError = $state<string | null>(null);
  let description = $state(store.t('props.defaultDescription'));
  let taskLimiterEnabled = $state(false);
  let taskLimitValue = $state(1);
  let taskLimitUnit = $state<SpeedLimitUnit>('MB/s');

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

  async function handleApplyLimit() {
    if (!task) return;
    const bps = taskLimiterEnabled ? unitToBps(taskLimitValue, taskLimitUnit) : null;
    await store.setTaskSpeedLimit(task.id, bps);
  }

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

  async function handleCopyUrl() {
    if (!task?.url) return;
    try {
      await navigator.clipboard.writeText(task.url);
      copiedUrl = true;
      setTimeout(() => {
        copiedUrl = false;
      }, 2000);
    } catch (e) {
      console.error("Copy URL failed:", e);
    }
  }

  async function handleMoveFile() {
    if (!task) return;
    moveError = null;
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: task.save_dir || undefined,
        title: store.t('addModal.selectFolder'),
      });

      if (selected && typeof selected === "string" && selected !== task.save_dir) {
        isMoving = true;
        await store.moveTaskFile(task.id, selected);
        moveSuccess = true;
        setTimeout(() => {
          moveSuccess = false;
        }, 3000);
      }
    } catch (e: any) {
      console.error("Move file error:", e);
      moveError = e?.message || store.t('props.moveError');
    } finally {
      isMoving = false;
    }
  }

  async function handleOpenFile() {
    if (!task?.file_path) return;
    await store.openFile(task.file_path);
  }

  async function handleOpenFolder() {
    if (!task?.file_path) return;
    await store.openFolder(task.file_path);
  }

  const hostname = $derived.by(() => {
    if (!task?.url) return "";
    try {
      return new URL(task.url).hostname;
    } catch {
      return task.url;
    }
  });
</script>

{#if isOpen && task}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-3 sm:p-4 bg-black/50 backdrop-blur-[2px] select-none animate-in fade-in duration-150"
  >
    <!-- Modal Dialog Window (Stitch Screen 8deae5ec187f4d6189939ff1da48c2bb) -->
    <div
      class="relative w-full max-w-2xl bg-[#0f141c] border border-[#252a33] rounded-2xl shadow-[0_24px_64px_-12px_rgba(0,0,0,0.85)] overflow-hidden flex flex-col transition-all duration-200"
      id="file-properties-modal"
    >
      <!-- Top Colored Accent Line -->
      <div
        class="h-[2px] w-full bg-gradient-to-r from-transparent via-[#4cd7f6] to-[#00e5ff]"
      ></div>

      <!-- Titlebar / Window Chrome -->
      <header
        data-tauri-drag-region
        class="h-10 px-4 bg-[#171c24] flex items-center justify-between border-b border-[#252a33] cursor-move"
      >
        <div class="flex items-center gap-2 min-w-0">
          <div
            class="w-2.5 h-2.5 rounded-full bg-[#4cd7f6] shadow-[0_0_8px_rgba(76,215,246,0.6)]"
          ></div>
          <span class="text-xs font-semibold text-white tracking-wide">
            IDM Turbo Desktop
          </span>
          <span
            class="text-[10px] font-mono text-slate-400 px-1.5 py-0.5 rounded bg-[#252a33] uppercase"
          >
            {store.t('props.title')}
          </span>
        </div>

        <button
          onclick={() => store.closePropertiesModal()}
          class="w-7 h-7 flex items-center justify-center rounded-lg text-slate-400 hover:text-slate-100 hover:bg-[#252a33] transition-colors cursor-pointer"
          title={store.t('common.close')}
          type="button"
        >
          <X class="w-4 h-4" />
        </button>
      </header>

      <!-- Modal Body (Scrollable) -->
      <div class="p-5 flex flex-col gap-4 max-h-[80vh] overflow-y-auto">
        <!-- Media Identification Bar -->
        <div
          class="p-4 rounded-xl bg-[#171c24] border border-[#252a33] flex items-start gap-4 shadow-sm"
        >
          <!-- Acrylic Cone / Stream Badge -->
          <div
            class="relative shrink-0 w-12 h-12 rounded-xl bg-[#252a33] border border-[#30353e] flex items-center justify-center shadow-md overflow-hidden text-[#00e5ff]"
          >
            <FileIcon class="w-6 h-6" />
            <div
              class="absolute bottom-1 right-1 w-2.5 h-2.5 rounded-full bg-[#10b981] shadow-[0_0_8px_rgba(16,185,129,0.8)]"
            ></div>
          </div>

          <!-- File Title & Instant Status -->
          <div class="flex-1 min-w-0">
            <div class="flex flex-wrap items-center gap-2 mb-1">
              {#if task.status === "completed"}
                <span
                  class="text-[10px] font-mono font-bold text-[#10b981] bg-[#10b981]/10 border border-[#10b981]/25 px-2 py-0.5 rounded-full flex items-center gap-1"
                >
                  <CheckCircle2 class="w-3 h-3" />
                  {store.t('props.completedBadge')}
                </span>
              {:else}
                <span
                  class="text-[10px] font-mono font-bold text-[#00e5ff] bg-[#00e5ff]/10 border border-[#00e5ff]/25 px-2 py-0.5 rounded-full flex items-center gap-1"
                >
                  <Clock class="w-3 h-3" />
                  {typeof task.status === 'string' ? task.status.toUpperCase() : store.t('common.failed').toUpperCase()}
                </span>
              {/if}
              <span class="text-[11px] font-mono text-slate-400">
                SHA-256 Validated
              </span>
              {#if task.completed_at}
                <span class="text-[11px] font-mono text-slate-500 ml-auto">
                  {task.completed_at}
                </span>
              {/if}
            </div>

            <h1
              class="text-sm font-bold text-white truncate leading-tight"
              title={task.filename}
            >
              {task.filename}
            </h1>
            <p class="text-xs text-slate-400 truncate mt-1">
              {store.t('props.savedInLocal', { count: task.connections })}
            </p>
          </div>
        </div>

        <!-- Primary Metadata Micro-Grid (4 tiles) -->
        <div
          class="grid grid-cols-2 sm:grid-cols-4 gap-2.5 p-2.5 bg-[#090e16] border border-[#252a33] rounded-xl shadow-inner"
        >
          <div class="p-2.5 rounded-lg bg-[#171c24] border border-[#252a33]/60 flex flex-col">
            <span class="text-[10px] font-mono text-slate-400 uppercase tracking-wider">
              {store.t('props.fileType')}
            </span>
            <span class="text-xs font-semibold text-white truncate mt-1 uppercase">
              {task.category}
            </span>
          </div>

          <div class="p-2.5 rounded-lg bg-[#171c24] border border-[#252a33]/60 flex flex-col">
            <span class="text-[10px] font-mono text-slate-400 uppercase tracking-wider">
              {store.t('props.fullSize')}
            </span>
            <span class="text-xs font-mono font-bold text-[#4cd7f6] truncate mt-1">
              {formatBytes(task.total_bytes || task.downloaded_bytes)}
            </span>
          </div>

          <div class="p-2.5 rounded-lg bg-[#171c24] border border-[#252a33]/60 flex flex-col">
            <span class="text-[10px] font-mono text-slate-400 uppercase tracking-wider">
              {store.t('props.threadConnections')}
            </span>
            <span class="text-xs font-mono font-medium text-white truncate mt-1">
              {store.t('props.tracksTurbo', { count: task.connections })}
            </span>
          </div>

          <div class="p-2.5 rounded-lg bg-[#171c24] border border-[#252a33]/60 flex flex-col">
            <span class="text-[10px] font-mono text-slate-400 uppercase tracking-wider">
              {store.t('props.partStatus')}
            </span>
            <span class="text-xs font-semibold text-[#10b981] truncate mt-1">
              {task.status === "completed" ? store.t('props.completed100') : store.t('props.partCache')}
            </span>
          </div>
        </div>

        <!-- Storage Path (Simpan Ke) & Move Button -->
        <div class="flex flex-col gap-1.5">
          <div class="flex items-center justify-between text-xs">
            <label
              for="prop-save-path"
              class="font-medium text-white flex items-center gap-1.5"
            >
              <FolderOpen class="w-3.5 h-3.5 text-[#4cd7f6]" />
              {store.t('props.saveTo')}
            </label>
            <span class="font-mono text-[11px] text-slate-400">
              {task.save_dir}
            </span>
          </div>

          <div class="flex items-center gap-2">
            <input
              id="prop-save-path"
              type="text"
              readonly
              value={task.file_path}
              class="w-full h-9 bg-[#090e16] border border-[#252a33] text-[#00e5ff] font-mono text-xs px-3 rounded-lg outline-none select-all truncate"
            />
            <button
              onclick={handleMoveFile}
              disabled={isMoving}
              class="h-9 px-3.5 rounded-lg bg-[#252a33] hover:bg-[#30353e] text-white text-xs font-semibold transition-all flex items-center gap-1.5 shrink-0 cursor-pointer active:scale-95 disabled:opacity-50"
              type="button"
            >
              <MoveRight class="w-3.5 h-3.5" />
              <span>{isMoving ? store.t('props.moving') : store.t('props.move')}</span>
            </button>
          </div>

          {#if moveSuccess}
            <div class="text-[11px] text-[#10b981] flex items-center gap-1 font-mono pt-0.5">
              <Check class="w-3.5 h-3.5" /> {store.t('props.moveSuccess')}
            </div>
          {/if}
          {#if moveError}
            <div class="text-[11px] text-[#ff5252] font-mono pt-0.5">
              {moveError}
            </div>
          {/if}
        </div>

        <!-- Source Direct URL (Alamat Unduh) -->
        <div class="flex flex-col gap-1.5">
          <div class="flex items-center justify-between text-xs">
            <label
              for="prop-url"
              class="font-medium text-white flex items-center gap-1.5"
            >
              <Link class="w-3.5 h-3.5 text-[#00e5ff]" />
              {store.t('props.directUrl')}
            </label>
            {#if copiedUrl}
              <span class="text-[#10b981] text-[11px] font-mono flex items-center gap-1">
                <Check class="w-3 h-3" /> {store.t('common.copied')}
              </span>
            {/if}
          </div>

          <div class="flex items-center gap-2">
            <input
              id="prop-url"
              type="text"
              readonly
              value={task.url}
              class="w-full h-9 bg-[#090e16] border border-[#252a33] text-slate-300 font-mono text-xs px-3 rounded-lg outline-none select-all truncate"
            />
            <button
              onclick={handleCopyUrl}
              class="h-9 w-9 rounded-lg bg-[#252a33] hover:bg-[#30353e] text-slate-300 hover:text-white flex items-center justify-center transition-colors shrink-0 cursor-pointer active:scale-95"
              title={store.t('common.copy')}
              type="button"
            >
              <Copy class="w-4 h-4" />
            </button>
          </div>
        </div>

        <!-- Deskripsi (Optional Notes) -->
        <div class="flex flex-col gap-1.5">
          <label
            for="prop-desc"
            class="text-xs font-medium text-white flex items-center gap-1.5"
          >
            <FileCode class="w-3.5 h-3.5 text-slate-400" />
            {store.t('props.description')}
          </label>
          <input
            id="prop-desc"
            type="text"
            bind:value={description}
            placeholder={store.t('props.descPlaceholder')}
            class="w-full h-9 bg-[#090e16] border border-[#252a33] text-slate-200 text-xs px-3 rounded-lg outline-none focus:border-[#00e5ff] transition-all"
          />
        </div>

        <!-- Batas Kecepatan Unduhan (Speed Limiter) -->
        <div class="p-3.5 rounded-xl bg-[#171c24] border border-[#252a33] flex flex-col gap-2.5 shadow-sm text-xs">
          <div class="flex items-center justify-between">
            <label class="font-medium text-white flex items-center gap-1.5 cursor-pointer">
              <Gauge class="w-3.5 h-3.5 text-[#00e5ff]" />
              {store.t('props.limitThisFile')}
            </label>
            <label class="flex items-center gap-2 cursor-pointer select-none">
              <input
                type="checkbox"
                bind:checked={taskLimiterEnabled}
                onchange={handleApplyLimit}
                class="w-4 h-4 rounded bg-[#090e16] border-[#252a33] text-[#00e5ff] focus:ring-0 cursor-pointer"
              />
              <span class="text-xs font-mono {taskLimiterEnabled ? 'text-[#00e5ff] font-semibold' : 'text-slate-400'}">
                {taskLimiterEnabled ? store.t('common.on') : store.t('common.off')}
              </span>
            </label>
          </div>

          {#if taskLimiterEnabled}
            <div class="flex items-center gap-2 pt-1 border-t border-[#252a33] flex-wrap">
              <input
                type="number"
                min="1"
                step="any"
                bind:value={taskLimitValue}
                oninput={handleApplyLimit}
                class="w-24 h-8 px-2.5 bg-[#090e16] border border-[#252a33] text-white font-mono text-xs rounded-lg focus:border-[#00e5ff] outline-none"
              />

              <div class="flex rounded-lg bg-[#090e16] p-0.5 border border-[#252a33]">
                <button
                  type="button"
                  onclick={() => { taskLimitUnit = 'KB/s'; handleApplyLimit(); }}
                  class="px-2.5 py-1 text-[11px] font-mono font-medium rounded-md transition-colors {taskLimitUnit === 'KB/s' ? 'bg-[#00e5ff] text-slate-950 font-bold' : 'text-slate-400 hover:text-white'}"
                >
                  KB/s
                </button>
                <button
                  type="button"
                  onclick={() => { taskLimitUnit = 'MB/s'; handleApplyLimit(); }}
                  class="px-2.5 py-1 text-[11px] font-mono font-medium rounded-md transition-colors {taskLimitUnit === 'MB/s' ? 'bg-[#00e5ff] text-slate-950 font-bold' : 'text-slate-400 hover:text-white'}"
                >
                  MB/s
                </button>
              </div>

              <!-- Quick presets -->
              <div class="flex items-center gap-1 sm:ml-auto">
                <button
                  type="button"
                  onclick={() => { taskLimitValue = 500; taskLimitUnit = 'KB/s'; handleApplyLimit(); }}
                  class="px-2 py-1 rounded-md bg-[#252a33] hover:bg-[#30353e] text-[10px] font-mono text-slate-300 hover:text-[#00e5ff]"
                >
                  500 KB/s
                </button>
                <button
                  type="button"
                  onclick={() => { taskLimitValue = 1; taskLimitUnit = 'MB/s'; handleApplyLimit(); }}
                  class="px-2 py-1 rounded-md bg-[#252a33] hover:bg-[#30353e] text-[10px] font-mono text-slate-300 hover:text-[#00e5ff]"
                >
                  1 MB/s
                </button>
                <button
                  type="button"
                  onclick={() => { taskLimitValue = 2; taskLimitUnit = 'MB/s'; handleApplyLimit(); }}
                  class="px-2 py-1 rounded-md bg-[#252a33] hover:bg-[#30353e] text-[10px] font-mono text-slate-300 hover:text-[#00e5ff]"
                >
                  2 MB/s
                </button>
              </div>
            </div>
          {/if}
        </div>

        <!-- Web Source & Referrer Group -->
        <div class="p-3.5 rounded-xl bg-[#171c24] border border-[#252a33] flex flex-col gap-2 shadow-sm text-xs">
          <div class="flex flex-col gap-0.5">
            <span class="text-[11px] font-mono text-slate-400">
              {store.t('props.hostWeb')}
            </span>
            <span class="text-xs font-medium text-[#4cd7f6] flex items-center gap-1 truncate">
              {hostname}
              <ExternalLink class="w-3 h-3 shrink-0" />
            </span>
          </div>
        </div>
      </div>

      <!-- Window Action Footer -->
      <footer
        class="px-5 py-3.5 bg-[#171c24] border-t border-[#252a33] flex items-center justify-between gap-2.5"
      >
        <!-- Buka Folder -->
        <button
          onclick={handleOpenFolder}
          class="px-3.5 py-2 rounded-xl text-xs font-semibold text-slate-300 bg-[#252a33] hover:bg-[#30353e] hover:text-white transition-all flex items-center gap-1.5 cursor-pointer shadow-sm active:scale-95"
          type="button"
        >
          <FolderOpen class="w-3.5 h-3.5 text-[#4cd7f6]" />
          <span>{store.t('dup.btnOpenFolder')}</span>
        </button>

        <div class="flex items-center gap-2">
          <button
            onclick={() => store.closePropertiesModal()}
            class="px-4 py-2 rounded-xl text-xs font-semibold text-slate-400 hover:text-slate-200 hover:bg-[#252a33] transition-all cursor-pointer"
            type="button"
          >
            {store.t('common.close')}
          </button>
          <button
            onclick={handleOpenFile}
            class="px-5 py-2 rounded-xl text-xs font-bold text-slate-950 bg-gradient-to-r from-[#00e5ff] to-[#4d8eff] hover:brightness-110 shadow-[0_4px_16px_rgba(0,229,255,0.25)] transition-all flex items-center gap-1.5 cursor-pointer active:scale-95"
            type="button"
          >
            <Play class="w-3.5 h-3.5 fill-current" />
            <span>{store.t('dup.btnOpenFile')}</span>
          </button>
        </div>
      </footer>
    </div>
  </div>
{/if}
