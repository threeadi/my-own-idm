<script lang="ts">
  import { store } from '$lib/idmStore.svelte';
  import { formatBytes } from '$lib/types';
  import {
    X,
    Trash2,
    AlertTriangle,
    Folder,
    FileText,
    Video,
    Music,
    Archive,
    Cpu,
    HardDrive
  } from '@lucide/svelte';

  const task = $derived(store.deleteModalTask);
  const isDownloading = $derived(task?.status === 'downloading');
  const isCompleted = $derived(task?.status === 'completed');
  const isPaused = $derived(task?.status === 'paused');
  const Icon = $derived(getFileIcon(task?.category));

  function getFileIcon(category?: string) {
    switch (category) {
      case 'video': return Video;
      case 'audio': return Music;
      case 'compressed': return Archive;
      case 'programs': return Cpu;
      case 'documents': return FileText;
      default: return Folder;
    }
  }

  function handleCancel() {
    store.closeDeleteModal();
  }

  async function handleConfirm() {
    await store.confirmDelete();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!store.isDeleteModalOpen) return;
    if (e.key === 'Escape') {
      handleCancel();
    } else if (e.key === 'Enter') {
      handleConfirm();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if store.isDeleteModalOpen && task}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-3 sm:p-4 bg-black/60 backdrop-blur-[3px] select-none animate-in fade-in duration-150">
    <!-- Obsidian Compact Dialog Container -->
    <div
      id="delete-confirm-modal"
      class="relative w-full max-w-md bg-[#171c24]/95 backdrop-blur-2xl rounded-xl shadow-2xl border border-[#30353e] overflow-hidden flex flex-col transition-all duration-200 animate-in zoom-in-95"
    >
      <!-- Top Red Glow Accent Line -->
      <div class="absolute inset-x-0 top-0 h-[2px] bg-gradient-to-r from-transparent via-[#ff5252]/80 to-transparent"></div>

      <!-- Window Title Bar -->
      <div data-tauri-drag-region class="px-4 py-2.5 bg-[#252a33]/90 flex items-center justify-between border-b border-[#30353e]/80 cursor-move">
        <div class="flex items-center gap-2">
          <Trash2 class="w-4 h-4 text-[#ff5252]" />
          <span class="font-sans text-xs font-semibold text-[#dee2ee] tracking-wide">
            {store.t('deleteModal.title')}
          </span>
        </div>

        <button
          onclick={handleCancel}
          class="w-6 h-6 rounded flex items-center justify-center text-[#8c909f] hover:text-[#dee2ee] hover:bg-[#30353e] transition-colors cursor-pointer"
          aria-label={store.t('common.close')}
          type="button"
        >
          <X class="w-3.5 h-3.5" />
        </button>
      </div>

      <!-- Modal Body -->
      <div class="p-5 space-y-4">
        <!-- Header Prompt -->
        <div class="flex items-start gap-3">
          <div class="w-10 h-10 rounded-xl bg-[#ff5252]/10 border border-[#ff5252]/30 flex items-center justify-center text-[#ff5252] shrink-0 shadow-[0_0_12px_rgba(255,82,82,0.2)]">
            <Trash2 class="w-5 h-5" />
          </div>
          <div>
            <h3 class="font-sans text-sm font-bold text-[#dee2ee] leading-snug">
              {store.t('deleteModal.confirmQuestion')}
            </h3>
            <p class="font-sans text-xs text-[#8c909f] mt-0.5 leading-relaxed">
              {store.t('deleteModal.confirmSubtitle')}
            </p>
          </div>
        </div>

        <!-- Target File Info Card -->
        <div class="p-3.5 rounded-xl bg-[#090e16]/80 border border-[#30353e] space-y-2.5">
          <div class="flex items-start gap-2.5">
            <div class="w-8 h-8 rounded-lg bg-[#252a33] flex items-center justify-center text-[#adc6ff] shrink-0 border border-[#30353e]">
              <Icon class="w-4 h-4" />
            </div>
            <div class="min-w-0 flex-1">
              <h4 class="font-sans text-xs font-semibold text-[#dee2ee] truncate" title={task.filename}>
                {task.filename}
              </h4>
              <div class="flex items-center gap-2 mt-1 text-[11px] font-mono text-[#8c909f]">
                <span>
                  {formatBytes(task.downloaded_bytes)}
                  {#if task.total_bytes}
                    <span class="text-[#8c909f]/60">/ {formatBytes(task.total_bytes)}</span>
                  {/if}
                </span>
                <span>•</span>
                <span class="text-[10px] px-1.5 py-0.2 rounded font-medium {isCompleted ? 'bg-[#10b981]/20 text-[#4edea3]' : isDownloading ? 'bg-[#03b5d3]/20 text-[#4cd7f6]' : isPaused ? 'bg-[#f59e0b]/20 text-[#f59e0b]' : 'bg-[#93000a]/20 text-[#ffb4ab]'}">
                  {isDownloading ? store.t('sidebar.statusDownloading') : isCompleted ? store.t('sidebar.statusCompleted') : isPaused ? store.t('sidebar.statusPaused') : store.t('common.failed')}
                </span>
              </div>
            </div>
          </div>

          {#if task.file_path}
            <div class="flex items-center gap-1.5 text-[10px] font-mono text-[#8c909f] pt-1 border-t border-[#252a33] truncate">
              <HardDrive class="w-3 h-3 text-[#4cd7f6] shrink-0" />
              <span class="truncate" title={task.file_path}>{task.file_path}</span>
            </div>
          {/if}
        </div>

        <!-- Warning If Actively Downloading -->
        {#if isDownloading}
          <div class="p-3 rounded-lg bg-[#f59e0b]/10 border border-[#f59e0b]/30 flex items-start gap-2.5 text-xs text-[#f59e0b]">
            <AlertTriangle class="w-4 h-4 shrink-0 mt-0.5" />
            <p class="text-[11px] leading-relaxed">
              {store.t('deleteModal.warningActive')}
            </p>
          </div>
        {/if}

        <!-- Checkbox: Also delete physical file from disk -->
        <label class="flex items-start gap-3 p-3 rounded-xl bg-[#1b2028] border border-[#252a33] hover:border-[#30353e] cursor-pointer group transition-colors">
          <input
            type="checkbox"
            bind:checked={store.deleteModalDeleteFile}
            class="mt-0.5 rounded border-[#30353e] bg-[#090e16] text-[#ff5252] focus:ring-0 w-4 h-4 cursor-pointer"
          />
          <div class="flex flex-col select-none">
            <span class="text-xs font-semibold text-[#dee2ee] group-hover:text-white transition-colors">
              {store.t('deleteModal.deleteFromDisk')}
            </span>
            <span class="text-[11px] text-[#8c909f] mt-0.5 leading-relaxed">
              {store.t('deleteModal.deleteFromDiskDesc')}
            </span>
          </div>
        </label>
      </div>

      <!-- Footer Action Buttons -->
      <div class="px-5 py-3.5 bg-[#171c24] border-t border-[#252a33] flex items-center justify-end gap-2.5">
        <button
          type="button"
          onclick={handleCancel}
          class="px-4 py-2 rounded-xl bg-[#252a33] hover:bg-[#30353e] text-[#dee2ee] font-medium text-xs transition-colors cursor-pointer border border-[#30353e]"
        >
          {store.t('deleteModal.btnCancel')}
        </button>

        <button
          type="button"
          onclick={handleConfirm}
          class="px-5 py-2 rounded-xl bg-[#ff5252] hover:bg-[#ef4444] text-white font-bold text-xs flex items-center gap-1.5 transition-all shadow-[0_0_14px_rgba(255,82,82,0.35)] cursor-pointer active:scale-95"
        >
          <Trash2 class="w-3.5 h-3.5 stroke-[2.5]" />
          <span>{store.t('deleteModal.btnDelete')}</span>
        </button>
      </div>
    </div>
  </div>
{/if}
