<script lang="ts">
  import { store } from '$lib/idmStore.svelte';
  import { formatBytes } from '$lib/types';
  import {
    AlertTriangle,
    CheckCircle2,
    X,
    ExternalLink,
    FolderOpen,
    Play,
    RotateCcw,
    CopyPlus
  } from '@lucide/svelte';

  const data = $derived(store.duplicateModalData);
  const isCompleted = $derived(data?.status === 'completed');
  const isPaused = $derived(data?.status === 'paused');

  function handleClose() {
    store.closeDuplicateModal();
  }

  function handleOpenTransfer() {
    if (data?.task_id) {
      const id = data.task_id;
      store.closeDuplicateModal();
      store.openTransferWindow(id);
    }
  }

  async function handleResumeTask() {
    if (data?.task_id) {
      const id = data.task_id;
      await store.resumeTask(id);
      store.closeDuplicateModal();
      store.openTransferWindow(id);
    }
  }

  async function handleOpenFile() {
    if (data?.file_path) {
      await store.openFile(data.file_path);
      store.closeDuplicateModal();
    }
  }

  async function handleOpenFolder() {
    if (data?.file_path) {
      await store.openFolder(data.file_path);
    }
  }

  function handleProceedAsNew() {
    store.proceedWithNewDownload();
  }
</script>

{#if store.isDuplicateModalOpen && data}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-3 sm:p-4 bg-black/60 backdrop-blur-[3px] select-none animate-in fade-in duration-150">
    <!-- Obsidian Compact Dialog Container -->
    <div
      id="duplicate-confirm-modal"
      class="relative w-full max-w-md bg-[#171c24]/95 backdrop-blur-2xl rounded-2xl shadow-2xl border border-[#30353e] overflow-hidden flex flex-col transition-all duration-200 animate-in zoom-in-95"
    >
      <!-- Top Glow Highlight Line -->
      <div
        class="absolute inset-x-0 top-0 h-[2px] bg-gradient-to-r from-transparent {isCompleted ? 'via-[#10b981]/80' : 'via-[#f59e0b]/80'} to-transparent"
      ></div>

      <!-- Window Title Bar -->
      <div data-tauri-drag-region class="px-4 py-2.5 bg-[#252a33]/90 flex items-center justify-between border-b border-[#30353e]/80 cursor-move">
        <div class="flex items-center gap-2">
          {#if isCompleted}
            <div class="w-4 h-4 rounded-full bg-[#10b981]/20 flex items-center justify-center text-[#4edea3]">
              <CheckCircle2 class="w-3.5 h-3.5" />
            </div>
            <span class="font-sans text-xs font-semibold text-[#dee2ee]">
              {store.t('dup.modalTitleCompleted')}
            </span>
          {:else}
            <div class="w-4 h-4 rounded-full bg-[#f59e0b]/20 flex items-center justify-center text-[#fbbf24]">
              <AlertTriangle class="w-3.5 h-3.5" />
            </div>
            <span class="font-sans text-xs font-semibold text-[#dee2ee]">
              {isPaused ? store.t('dup.modalTitlePaused') : store.t('dup.modalTitleActive')}
            </span>
          {/if}
        </div>

        <button
          onclick={handleClose}
          class="w-6 h-6 rounded flex items-center justify-center text-[#8c909f] hover:text-[#dee2ee] hover:bg-[#30353e] transition-colors cursor-pointer"
          aria-label={store.t('common.close')}
          type="button"
        >
          <X class="w-3.5 h-3.5" />
        </button>
      </div>

      <!-- Modal Body -->
      <div class="p-4 sm:p-5 space-y-4">
        <!-- File Info Card -->
        <div class="p-3 rounded-xl bg-[#090e16]/80 border border-[#252a33] flex items-start gap-3">
          <div class="w-10 h-10 rounded-xl {isCompleted ? 'bg-[#10b981]/15 text-[#4edea3] border border-[#10b981]/30' : 'bg-[#f59e0b]/15 text-[#fbbf24] border border-[#f59e0b]/30'} flex shrink-0 items-center justify-center">
            {#if isCompleted}
              <CheckCircle2 class="w-5 h-5" />
            {:else}
              <AlertTriangle class="w-5 h-5" />
            {/if}
          </div>

          <div class="flex-1 min-w-0 space-y-1">
            <h3 class="font-sans text-xs font-bold text-[#dee2ee] truncate" title={data.filename || ''}>
              {data.filename || store.t('dup.unknownName')}
            </h3>

            {#if !isCompleted}
              <!-- Mini Telemetry Progress Bar -->
              <div class="space-y-1 pt-1">
                <div class="flex items-center justify-between text-[10px] font-mono text-[#8c909f]">
                  <span>{store.t('dup.sizeFrom', { downloaded: formatBytes(data.downloaded_bytes), total: data.total_bytes ? formatBytes(data.total_bytes) : store.t('dup.sizeUncertain') })}</span>
                  <span class="text-[#fbbf24] font-bold">{data.percent.toFixed(1)}%</span>
                </div>
                <div class="w-full h-1.5 bg-[#171c24] rounded-full overflow-hidden border border-[#30353e]/40">
                  <div
                    class="h-full bg-gradient-to-r from-[#f59e0b] to-[#fbbf24] rounded-full transition-all duration-300"
                    style="width: {data.percent}%"
                  ></div>
                </div>
              </div>
            {:else}
              <div class="flex items-center gap-2 text-[11px] font-mono text-[#8c909f] pt-0.5">
                <span class="text-[#4edea3] font-medium">{store.t('dup.sizeLabel', { size: formatBytes(data.downloaded_bytes) })}</span>
                {#if data.completed_at}
                  <span>•</span>
                  <span>{data.completed_at}</span>
                {/if}
              </div>
            {/if}
          </div>
        </div>

        <!-- Description Message -->
        <p class="font-sans text-xs text-[#c2c6d6] leading-relaxed">
          {#if isCompleted}
            {store.t('dup.descCompleted')}
          {:else if isPaused}
            {store.t('dup.descPaused')}
          {:else}
            {store.t('dup.descActive')}
          {/if}
        </p>

        {#if isCompleted && data.file_path}
          <div class="p-2 rounded-lg bg-[#090e16]/60 border border-[#30353e]/80 text-[11px] font-mono text-[#8c909f] truncate flex items-center gap-1.5">
            <FolderOpen class="w-3.5 h-3.5 text-[#4cd7f6] shrink-0" />
            <span class="truncate text-[#dee2ee]" title={data.file_path}>{data.file_path}</span>
          </div>
        {/if}
      </div>

      <!-- Action Footer -->
      <div class="px-4 sm:px-5 py-3 bg-[#252a33]/60 border-t border-[#30353e]/80 flex items-center justify-between gap-2 flex-wrap">
        <button
          onclick={handleClose}
          class="h-8 px-3 rounded-lg text-[#8c909f] hover:text-[#dee2ee] hover:bg-[#252a33] text-xs font-medium transition-colors cursor-pointer"
          type="button"
        >
          {store.t('common.cancel')}
        </button>

        <div class="flex items-center gap-2 flex-wrap">
          {#if isCompleted}
            {#if data.file_path}
              <button
                onclick={handleOpenFolder}
                class="h-8 px-3 rounded-lg bg-[#252a33] hover:bg-[#343942] text-[#dee2ee] border border-[#30353e] text-xs font-medium flex items-center gap-1.5 transition-colors cursor-pointer"
                type="button"
              >
                <FolderOpen class="w-3.5 h-3.5 text-[#4cd7f6]" />
                <span>{store.t('dup.btnOpenFolder')}</span>
              </button>
              <button
                onclick={handleOpenFile}
                class="h-8 px-3.5 rounded-lg bg-[#10b981] hover:bg-[#059669] text-white text-xs font-bold flex items-center gap-1.5 shadow-[0_0_15px_rgba(16,185,129,0.35)] transition-all cursor-pointer"
                type="button"
              >
                <CheckCircle2 class="w-3.5 h-3.5" />
                <span>{store.t('dup.btnOpenFile')}</span>
              </button>
            {/if}
            <button
              onclick={handleProceedAsNew}
              class="h-8 px-3 rounded-lg bg-[#4d8eff]/15 hover:bg-[#4d8eff]/25 text-[#adc6ff] border border-[#4d8eff]/30 text-xs font-semibold flex items-center gap-1.5 transition-colors cursor-pointer"
              title={store.t('dup.titleNewCopy')}
              type="button"
            >
              <RotateCcw class="w-3 h-3" />
              <span>{store.t('dup.btnRedownload')}</span>
            </button>
          {:else}
            <button
              onclick={handleProceedAsNew}
              class="h-8 px-3 rounded-lg bg-[#252a33] hover:bg-[#343942] text-[#dee2ee] border border-[#30353e] text-xs font-medium flex items-center gap-1.5 transition-colors cursor-pointer"
              title={store.t('dup.titleAutoRename')}
              type="button"
            >
              <CopyPlus class="w-3.5 h-3.5 text-[#4cd7f6]" />
              <span>{store.t('dup.btnDownloadNew')}</span>
            </button>

            {#if isPaused}
              <button
                onclick={handleResumeTask}
                class="h-8 px-3.5 rounded-lg bg-[#4edea3] hover:bg-[#10b981] text-[#003822] text-xs font-bold flex items-center gap-1.5 shadow-[0_0_15px_rgba(78,222,163,0.35)] transition-all cursor-pointer"
                type="button"
              >
                <Play class="w-3.5 h-3.5 fill-current" />
                <span>{store.t('dup.btnResume')}</span>
              </button>
            {:else}
              <button
                onclick={handleOpenTransfer}
                class="h-8 px-3.5 rounded-lg bg-gradient-to-r from-[#00e5ff] to-[#4cd7f6] text-[#00363d] hover:brightness-110 text-xs font-bold flex items-center gap-1.5 shadow-[0_0_15px_rgba(0,229,255,0.35)] active:scale-95 transition-all cursor-pointer"
                type="button"
              >
                <ExternalLink class="w-3.5 h-3.5" />
                <span>{store.t('dup.btnViewProgress')}</span>
              </button>
            {/if}
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}
